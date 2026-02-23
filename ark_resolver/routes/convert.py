from urllib.parse import unquote

from opentelemetry.trace import Status
from opentelemetry.trace import StatusCode
from sanic import Blueprint
from sanic import HTTPResponse
from sanic import Request
from sanic import json
from sanic.log import logger

import ark_resolver.check_digit as check_digit_py
from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import ArkUrlFormatter
from ark_resolver.ark_url import ArkUrlInfo
from ark_resolver.ark_url_rust import ArkUrlFormatter as ArkUrlFormatterRust
from ark_resolver.ark_url_rust import ArkUrlInfo as ArkUrlInfoRust
from ark_resolver.error_diagnostics import classify_exception
from ark_resolver.error_diagnostics import diagnose_non_ark_path
from ark_resolver.error_diagnostics import error_response
from ark_resolver.error_diagnostics import pre_validate_ark
from ark_resolver.error_diagnostics import report_error_to_sentry
from ark_resolver.parallel_execution import parallel_executor
from ark_resolver.tracing import tracer

convert_bp = Blueprint("convert", url_prefix="/convert")


@convert_bp.get("/<ark_id:path>")
async def convert(req: Request, ark_id: str = "") -> HTTPResponse:
    """Ark V0 to V1 conversion endpoint with shadow execution"""

    # BR: Paths not starting with ark:/ are not ARK identifiers.
    # Skip Sentry reporting here to avoid noise from crawlers (favicon.ico, robots.txt, etc.)
    if not ark_id.startswith("ark:/"):
        diagnostic = diagnose_non_ark_path(ark_id)
        return error_response(diagnostic)

    # Decode the ARK ID (idempotent operation).
    ark_id_decoded = unquote(ark_id)

    with tracer.start_as_current_span("convert") as span:
        span.set_attribute("http.method", "GET")
        span.set_attribute("ark_id", ark_id_decoded)

        # BR: Pre-validate for common corruption patterns before parsing
        diagnostic = pre_validate_ark(ark_id_decoded)
        if diagnostic is not None:
            report_error_to_sentry("convert", diagnostic, ark_id_decoded)
            span.set_status(Status(StatusCode.ERROR, diagnostic.code.value))
            logger.error(f"Invalid ARK ID ({diagnostic.code.value}): {ark_id_decoded}")
            return error_response(diagnostic)

        try:
            # Shadow execution: run both Python and Rust implementations
            def python_convert():
                ark_url_info = ArkUrlInfo(req.app.config.settings, ark_id_decoded)
                resource_iri = ark_url_info.to_resource_iri()
                timestamp = ark_url_info.get_timestamp()
                return ArkUrlFormatter(req.app.config.settings).resource_iri_to_ark_id(resource_iri=resource_iri, timestamp=timestamp)

            def rust_convert():
                rust_settings = req.app.config.rust_settings
                if rust_settings is None:
                    raise RuntimeError("Rust settings not available")
                ark_url_info = ArkUrlInfoRust(rust_settings, ark_id_decoded)
                resource_iri = ark_url_info.to_resource_iri()
                timestamp = ark_url_info.get_timestamp()
                return ArkUrlFormatterRust(rust_settings).resource_iri_to_ark_id(resource_iri=resource_iri, timestamp=timestamp)

            # Execute both implementations in parallel
            converted_ark_id, execution_result = parallel_executor.execute_parallel("convert", python_convert, rust_convert)

            # Add parallel execution metrics to span
            parallel_executor.add_to_span(span, execution_result)

            # Track with Sentry
            parallel_executor.track_with_sentry(execution_result)

            span.set_status(Status(StatusCode.OK))  # Mark as successful

        except (ArkUrlException, check_digit_py.CheckDigitException, KeyError) as ex:
            diagnostic = classify_exception(ex, ark_id_decoded)
            report_error_to_sentry("convert", diagnostic, ark_id_decoded, exception=ex)
            span.set_status(Status(StatusCode.ERROR, diagnostic.code.value))
            logger.error(f"Invalid ARK ID ({diagnostic.code.value}): {ark_id_decoded}")
            return error_response(diagnostic)

        span.add_event("Convert result", {"convert_result": converted_ark_id})
        logger.info(f"Convert {ark_id_decoded} to {converted_ark_id}")
        return json(
            {"input": ark_id_decoded, "converted": converted_ark_id},
        )
