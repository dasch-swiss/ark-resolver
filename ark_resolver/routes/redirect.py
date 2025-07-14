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
from ark_resolver.tracing import tracer

redirect_bp = Blueprint("redirect", url_prefix="")


@redirect_bp.get("/<path:path>")
async def catch_all(_: Request, path: str = "") -> HTTPResponse:
    """
    Catch all URL. Tries to redirect the given ARK ID.
    """
    # Check if the path could be a valid ARK ID.
    if not path.startswith("ark:/"):
        msg = f"Invalid ARK ID: {path}"
        return response.text(body=msg, status=400)

    # Decode the ARK ID (idempotent operation).
    ark_id_decoded = unquote(path)

    with tracer.start_as_current_span("redirect") as span:
        span.set_attribute("ark_id", ark_id_decoded)  # Attach ARK ID as metadata

        try:
            redirect_url = ArkUrlInfo(settings=_.app.config.settings, ark_id=ark_id_decoded).to_redirect_url()
            span.set_status(Status(StatusCode.OK))  # Mark as successful

        except ArkUrlException as ex:
            span.set_status(Status(StatusCode.ERROR, "Invalid ARK ID"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}")
            return response.text(body=ex.message, status=400)

        except check_digit_py.CheckDigitException as ex:
            span.set_status(Status(StatusCode.ERROR, "Check Digit Error"))
            logger.error(f"Invalid ARK ID (wrong check digit): {ark_id_decoded}", exc_info=ex)
            return response.text(body=ex.message, status=400)

        except KeyError as ex:
            span.set_status(Status(StatusCode.ERROR, "KeyError (project not found)"))
            logger.error(f"Invalid ARK ID (project not found): {ark_id_decoded}", exc_info=ex)
            return response.text(body="Invalid ARK ID (project not found)", status=400)

        span.add_event("Redirecting", {"redirect_url": redirect_url})
        logger.info(f"Redirecting {ark_id_decoded} to {redirect_url}")
        return response.redirect(redirect_url)
