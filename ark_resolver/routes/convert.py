from urllib.parse import unquote

import sentry_sdk
from opentelemetry.trace import Status
from opentelemetry.trace import StatusCode
from sanic import Blueprint
from sanic import HTTPResponse
from sanic import Request
from sanic import json
from sanic import response
from sanic.log import logger

import ark_resolver.check_digit as check_digit_py
from ark_resolver._rust import load_settings as load_settings_rust
from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import ArkUrlFormatter
from ark_resolver.ark_url import ArkUrlInfo
from ark_resolver.ark_url_rust import ArkUrlFormatter as ArkUrlFormatterRust
from ark_resolver.ark_url_rust import ArkUrlInfo as ArkUrlInfoRust
from ark_resolver.parallel_execution import parallel_executor
from ark_resolver.tracing import tracer

convert_bp = Blueprint("convert", url_prefix="/convert")


@convert_bp.get("/<ark_id:path>")
async def convert(req: Request, ark_id: str = "") -> HTTPResponse:
    """Ark V0 to V1 conversion endpoint with shadow execution"""

    # Check if the path could be a valid ARK ID.
    if not ark_id.startswith("ark:/"):
        msg = f"Invalid ARK ID: {ark_id}"
        return response.text(body=msg, status=400)

    # Decode the ARK ID (idempotent operation).
    ark_id_decoded = unquote(ark_id)

    with tracer.start_as_current_span("convert") as span:
        span.set_attribute("http.method", "GET")
        span.set_attribute("ark_id", ark_id_decoded)  # Attach ARK ID as metadata

        try:
            # Shadow execution: run both Python and Rust implementations
            def python_convert():
                ark_url_info = ArkUrlInfo(req.app.config.settings, ark_id_decoded)
                resource_iri = ark_url_info.to_resource_iri()
                timestamp = ark_url_info.get_timestamp()
                return ArkUrlFormatter(req.app.config.settings).resource_iri_to_ark_id(resource_iri=resource_iri, timestamp=timestamp)

            def rust_convert():
                rust_settings = load_settings_rust()
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

        except ArkUrlException as ex:
            with sentry_sdk.push_scope() as scope:
                scope.fingerprint = ["convert", "invalid-ark-id"]
                scope.set_tag("ark_id", ark_id_decoded[:100])
                scope.set_tag("error_type", "ArkUrlException")
                sentry_sdk.capture_exception(ex)
            span.set_status(Status(StatusCode.ERROR, "Invalid ARK ID"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}")
            return response.text(body=ex.message, status=400)

        except check_digit_py.CheckDigitException as ex:
            with sentry_sdk.push_scope() as scope:
                scope.fingerprint = ["convert", "check-digit-error"]
                scope.set_tag("ark_id", ark_id_decoded[:100])
                scope.set_tag("error_type", "CheckDigitException")
                sentry_sdk.capture_exception(ex)
            span.set_status(Status(StatusCode.ERROR, "Check Digit Error"))
            logger.error(f"Invalid ARK ID (wrong check digit): {ark_id_decoded}", exc_info=ex)
            return response.text(body=ex.message, status=400)

        except KeyError as ex:
            with sentry_sdk.push_scope() as scope:
                scope.fingerprint = ["convert", "project-not-found"]
                scope.set_tag("ark_id", ark_id_decoded[:100])
                scope.set_tag("project_id", str(ex)[:10])
                sentry_sdk.capture_exception(ex)
            span.set_status(Status(StatusCode.ERROR, "KeyError (project not found)"))
            logger.error(f"Invalid ARK ID (project not found): {ark_id_decoded}", exc_info=ex)
            return response.text(body="Invalid ARK ID", status=400)

        span.add_event("Convert result", {"convert_result": converted_ark_id})
        logger.info(f"Convert {ark_id_decoded} to {converted_ark_id}")
        return json(
            {"input": ark_id_decoded, "converted": converted_ark_id},
        )
