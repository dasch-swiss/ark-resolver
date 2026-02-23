from urllib.parse import unquote

from opentelemetry.trace import Status
from opentelemetry.trace import StatusCode
from sanic import Blueprint
from sanic import HTTPResponse
from sanic import Request
from sanic import response
from sanic.log import logger

import ark_resolver.check_digit as check_digit_py
from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import ArkUrlInfo
from ark_resolver.ark_url_rust import ArkUrlInfo as ArkUrlInfoRust
from ark_resolver.error_diagnostics import classify_exception
from ark_resolver.error_diagnostics import diagnose_non_ark_path
from ark_resolver.error_diagnostics import error_response
from ark_resolver.error_diagnostics import pre_validate_ark
from ark_resolver.error_diagnostics import report_error_to_sentry
from ark_resolver.parallel_execution import parallel_executor
from ark_resolver.tracing import tracer

redirect_bp = Blueprint("redirect", url_prefix="")


@redirect_bp.get("/<path:path>")
async def catch_all(_: Request, path: str = "") -> HTTPResponse:
    """
    Catch all URL. Tries to redirect the given ARK ID.
    """
    # BR: Paths not starting with ark:/ are not ARK identifiers.
    # Skip Sentry reporting here to avoid noise from crawlers (favicon.ico, robots.txt, etc.)
    if not path.startswith("ark:/"):
        diagnostic = diagnose_non_ark_path(path)
        return error_response(diagnostic)

    # Decode the ARK ID (idempotent operation).
    ark_id_decoded = unquote(path)

    with tracer.start_as_current_span("redirect") as span:
        span.set_attribute("ark_id", ark_id_decoded)

        # BR: Pre-validate for common corruption patterns before parsing
        diagnostic = pre_validate_ark(ark_id_decoded)
        if diagnostic is not None:
            report_error_to_sentry("redirect", diagnostic, ark_id_decoded)
            span.set_status(Status(StatusCode.ERROR, diagnostic.code.value))
            logger.error(f"Invalid ARK ID ({diagnostic.code.value}): {ark_id_decoded}")
            return error_response(diagnostic)

        try:

            def python_redirect():
                return ArkUrlInfo(
                    settings=_.app.config.settings,
                    ark_id=ark_id_decoded,
                ).to_redirect_url()

            def rust_redirect():
                rust_settings = _.app.config.rust_settings
                if rust_settings is None:
                    raise RuntimeError("Rust settings not available")
                return ArkUrlInfoRust(rust_settings, ark_id_decoded).to_redirect_url()

            redirect_url, execution_result = parallel_executor.execute_parallel("redirect", python_redirect, rust_redirect)

            parallel_executor.add_to_span(span, execution_result)
            parallel_executor.track_with_sentry(execution_result)

            span.set_status(Status(StatusCode.OK))

        except (ArkUrlException, check_digit_py.CheckDigitException, KeyError) as ex:
            diagnostic = classify_exception(ex, ark_id_decoded)
            report_error_to_sentry("redirect", diagnostic, ark_id_decoded, exception=ex)
            span.set_status(Status(StatusCode.ERROR, diagnostic.code.value))
            logger.error(f"Invalid ARK ID ({diagnostic.code.value}): {ark_id_decoded}")
            return error_response(diagnostic)

        span.add_event("Redirecting", {"redirect_url": redirect_url})
        logger.info(f"Redirecting {ark_id_decoded} to {redirect_url}")
        return response.redirect(redirect_url)
