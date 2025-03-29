#!/usr/bin/env python3

# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0


#################################################################################################
# DSP ARK redirect server and conversion utility.
#
# For help on command-line options, run with --help.
#################################################################################################

import configparser
import hashlib
import hmac
import os
from asyncio import sleep
from io import StringIO
from typing import cast
from urllib.parse import unquote

import sentry_sdk
from opentelemetry import trace
from opentelemetry.propagate import set_global_textmap
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.trace import Status
from opentelemetry.trace import StatusCode
from sanic import HTTPResponse
from sanic import Sanic
from sanic import response
from sanic.log import logger
from sanic_cors import CORS  # type: ignore[import-untyped]
from sentry_sdk.integrations.opentelemetry import SentryPropagator
from sentry_sdk.integrations.opentelemetry import SentrySpanProcessor
from sentry_sdk.integrations.rust_tracing import RustTracingIntegration

import ark_resolver.check_digit as check_digit_py
import ark_resolver.health

# TODO: rust types don't seem to work with mypy
from ark_resolver import _rust  # type: ignore[attr-defined]
from ark_resolver.ark_settings import ArkUrlSettings
from ark_resolver.ark_settings import load_settings
from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import ArkUrlInfo

#################################################################################################
# OpenTelemetry

provider = TracerProvider()

# Add both Sentry and Console span processors to the provider
provider.add_span_processor(SentrySpanProcessor())  # Sentry integration

set_global_textmap(SentryPropagator())

# Sets the global default tracer provider
trace.set_tracer_provider(provider)

# Creates a tracer from the global tracer provider
tracer = trace.get_tracer(__name__)

#################################################################################################
# Server implementation.

Sanic.start_method = "fork"

app = Sanic("ark_resolver")
CORS(app)

# Register health check route
app.blueprint(ark_resolver.health.health_bp)


def app_settings() -> ArkUrlSettings:
    return cast(ArkUrlSettings, app.config.settings)


@app.before_server_start
async def init_sentry(_):
    sentry_dsn = os.environ.get("ARK_SENTRY_DSN", None)
    sentry_debug = os.environ.get("ARK_SENTRY_DEBUG", "False")
    sentry_environment = os.environ.get("ARK_SENTRY_ENVIRONMENT", None)
    sentry_release = os.environ.get("ARK_SENTRY_RELEASE", None)
    if sentry_dsn:
        sentry_sdk.init(
            dsn=sentry_dsn,
            debug=sentry_debug,
            environment=sentry_environment,
            release=sentry_release,
            # Add data like request headers and IP for users;
            # see https://docs.sentry.io/platforms/python/data-management/data-collected/ for more info
            send_default_pii=True,
            # Set traces_sample_rate to 1.0 to capture 100%
            # of transactions for tracing.
            traces_sample_rate=1.0,
            # Set profiles_sample_rate to 1.0 to profile 100%
            # of sampled transactions.
            # We recommend adjusting this value in production.
            profiles_sample_rate=1.0,
            instrumenter="otel",
            integrations=[
                RustTracingIntegration(
                    "_rust",
                    _rust.initialize_tracing,
                    include_tracing_fields=True,
                )
            ],
        )
        logger.info("Sentry initialized.")
    else:
        logger.info("No SENTRY_DSN found in environment variables. Sentry will not be initialized.")


def get_safe_config() -> str:
    """
    Returns the app's configuration
    """
    # Make a copy of the configuration.
    config_output = StringIO()
    app_settings().config.write(config_output)
    safe_config = configparser.ConfigParser()
    safe_config.read_string(config_output.getvalue())

    # Remove the GitHub secret from the copy.
    safe_config.remove_option("DEFAULT", "ArkGitHubSecret")

    # Return the result as a string.
    safe_config_output = StringIO()
    safe_config.write(safe_config_output)
    return safe_config_output.getvalue()


@app.get("/config")
async def safe_config_get(_) -> HTTPResponse:
    """
    Returns the app's configuration
    """
    return response.text(get_safe_config())


@app.head("/config")
async def safe_config_head(_) -> HTTPResponse:
    """
    Returns only the head of the config response
    """
    config_str = get_safe_config()

    headers = {"Content-Length": str(len(config_str)), "Content-Type": "text/plain; charset=utf-8"}

    return response.text("", headers=headers)


@app.post("/reload")
async def reload(req) -> HTTPResponse:
    """
    Requests reloading of the configuration. Checks if the request is authorized.
    """
    with tracer.start_as_current_span("reload") as span:
        span.set_attribute("request", req)  # Attach ARK ID as metadata

        # Get the signature submitted with the request.
        if "X-Hub-Signature" not in req.headers:
            return response.text("Unauthorized", status=401)

        signature_header = req.headers["X-Hub-Signature"]

        if not signature_header.startswith("sha1="):
            return response.text("Unauthorized", status=401)

        submitted_signature = signature_header.split("=")[1]

        # Compute a signature for the request using the configured GitHub secret.
        secret = app_settings().top_config["ArkGitHubSecret"]
        computed_signature = hmac.new(secret.encode(), req.body, hashlib.sha1).hexdigest()

        # If the submitted signature is the same as the computed one, the request is valid.
        if hmac.compare_digest(submitted_signature, computed_signature):
            # reload configuration right away
            reload_config()
            # reload configuration again in 5 minutes (non-blocking), when github caching has expired
            app.add_task(schedule_reload())
            span.set_status(Status(StatusCode.OK))
            return response.text("", status=204)
        else:
            span.set_status(Status(StatusCode.ERROR))
            return response.text("Unauthorized", status=401)


@app.get("/<path:path>")
async def catch_all(_, path="") -> HTTPResponse:
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
            redirect_url = ArkUrlInfo(settings=app_settings(), ark_id=ark_id_decoded).to_redirect_url()
            span.set_status(Status(StatusCode.OK))  # Mark as successful
        except ArkUrlException as ex:
            span.set_status(Status(StatusCode.ERROR, "Invalid ARK ID"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}")
            return response.text(body=ex.message, status=400)

        except check_digit_py.CheckDigitException as ex:
            span.set_status(Status(StatusCode.ERROR, "Check Digit Error"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}", exc_info=ex)
            return response.text(body=ex.message, status=400)

        except KeyError as ex:
            span.set_status(Status(StatusCode.ERROR, "KeyError"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}", exc_info=ex)
            return response.text(body="Invalid ARK ID", status=400)

        span.add_event("Redirecting", {"redirect_url": redirect_url})
        logger.info(f"Redirecting {ark_id_decoded} to {redirect_url}")
        return response.redirect(redirect_url)


def start_server(config_path: str, settings: ArkUrlSettings) -> None:
    """
    Starts the app as server with the given settings.
    """
    app.config.config_path = config_path
    app.config.settings = settings
    app.run(host=settings.top_config["ArkInternalHost"], port=settings.top_config.getint("ArkInternalPort"))


#################################################################################################
# schedule reload


async def schedule_reload() -> None:
    await sleep(5 * 60)
    logger.info("Reloading config again.")
    reload_config()


#################################################################################################
# Reload config


def reload_config() -> None:
    settings = load_settings(app.config.config_path)
    app.config.settings = settings
    logger.info("Configuration reloaded.")
