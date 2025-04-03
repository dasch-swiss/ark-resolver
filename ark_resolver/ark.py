#!/usr/bin/env python3

# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0


#################################################################################################
# DSP ARK redirect server and conversion utility.
#
# For help on command-line options, run with --help.
#################################################################################################

import argparse
import configparser
import hashlib
import hmac
import os
import sys
from asyncio import sleep
from io import StringIO
from typing import Any
from urllib.parse import unquote

import requests
import sentry_sdk
from opentelemetry.trace import Status
from opentelemetry.trace import StatusCode
from sanic import HTTPResponse
from sanic import Request
from sanic import Sanic
from sanic import response
from sanic.log import logger
from sanic_cors import CORS  # type: ignore[import-untyped]
from sentry_sdk.integrations.rust_tracing import RustTracingIntegration

import ark_resolver.check_digit as check_digit_py
import ark_resolver.routes.convert
import ark_resolver.routes.health
from ark_resolver import _rust  # type: ignore[attr-defined]
from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import ArkUrlFormatter
from ark_resolver.ark_url import ArkUrlInfo
from ark_resolver.ark_url import ArkUrlSettings
from ark_resolver.tracing import tracer

#################################################################################################
# Server implementation.

Sanic.start_method = "fork"

app = Sanic("ark_resolver")
CORS(app)

# Register health check route
app.blueprint(ark_resolver.routes.health.health_bp)
app.blueprint(ark_resolver.routes.convert.convert_bp)


@app.before_server_start
async def init_sentry(_: Any) -> None:
    sentry_dsn = os.environ.get("ARK_SENTRY_DSN", None)
    sentry_debug = os.environ.get("ARK_SENTRY_DEBUG", "False")
    sentry_environment = os.environ.get("ARK_SENTRY_ENVIRONMENT", None)
    sentry_release = os.environ.get("ARK_SENTRY_RELEASE", None)
    if sentry_dsn:
        sentry_sdk.init(
            dsn=sentry_dsn,
            debug=sentry_debug,  # type: ignore[arg-type]
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
    app.config.settings.config.write(config_output)
    safe_config = configparser.ConfigParser()
    safe_config.read_string(config_output.getvalue())

    # Remove the GitHub secret from the copy.
    safe_config.remove_option("DEFAULT", "ArkGitHubSecret")

    # Return the result as a string.
    safe_config_output = StringIO()
    safe_config.write(safe_config_output)
    return safe_config_output.getvalue()


@app.get("/config")
async def safe_config_get(_: Request) -> HTTPResponse:
    """
    Returns the app's configuration
    """
    return response.text(get_safe_config())


@app.head("/config")
async def safe_config_head(_: Request) -> HTTPResponse:
    """
    Returns only the head of the config response
    """
    config_str = get_safe_config()

    headers = {"Content-Length": str(len(config_str)), "Content-Type": "text/plain; charset=utf-8"}

    return response.text("", headers=headers)


@app.post("/reload")
async def reload(req: Request) -> HTTPResponse:
    """
    Requests reloading of the configuration. Checks if the request is authorized.
    """
    print(type(req))
    print(req)
    with tracer.start_as_current_span("reload") as span:
        span.set_attribute("request", "reload config")

        # Get the signature submitted with the request.
        if "X-Hub-Signature" not in req.headers:
            return response.text("Unauthorized", status=401)

        signature_header = req.headers["X-Hub-Signature"]

        if not signature_header.startswith("sha1="):
            return response.text("Unauthorized", status=401)

        submitted_signature = signature_header.split("=")[1]

        # Compute a signature for the request using the configured GitHub secret.
        secret = app.config.settings.top_config["ArkGitHubSecret"]
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
            redirect_url = ArkUrlInfo(settings=app.config.settings, ark_id=ark_id_decoded).to_redirect_url()
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
            span.set_status(Status(StatusCode.ERROR, "KeyError"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}", exc_info=ex)
            return response.text(body="Invalid ARK ID", status=400)

        span.add_event("Redirecting", {"redirect_url": redirect_url})
        logger.info(f"Redirecting {ark_id_decoded} to {redirect_url}")
        return response.redirect(redirect_url)


def server(settings: ArkUrlSettings) -> None:
    """
    Starts the app as server with the given settings.
    """
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


#################################################################################################
# Loading of config and registry files.


def load_settings(config_path: str) -> ArkUrlSettings:
    """
    Loads configuration from given path and returns an ArkUrlSettings.
    """
    app.config.config_path = config_path

    # Default configuration from environment variables.
    environment_vars = {
        "ArkExternalHost": os.environ.get("ARK_EXTERNAL_HOST", "ark.example.org"),
        "ArkInternalHost": os.environ.get("ARK_INTERNAL_HOST", "0.0.0.0"),
        "ArkInternalPort": os.environ.get("ARK_INTERNAL_PORT", "3336"),
        "ArkNaan": os.environ.get("ARK_NAAN", "00000"),
        "ArkHttpsProxy": os.environ.get("ARK_HTTPS_PROXY", "true"),
        "ArkRegistry": os.environ.get("ARK_REGISTRY", "ark-registry.ini"),
        "ArkGitHubSecret": os.environ.get("ARK_GITHUB_SECRET", ""),
    }

    # Read the config and registry files.
    config = configparser.ConfigParser(defaults=environment_vars)
    config.read_file(open(config_path))

    registry_path = config["DEFAULT"]["ArkRegistry"]

    if registry_path.startswith("http"):
        registry_str = requests.get(registry_path, timeout=10).text
        config.read_string(registry_str, source=registry_path)
    else:
        config.read_file(open(registry_path))

    settings = ArkUrlSettings(config)

    return settings


#################################################################################################
# Command-line invocation.


def main() -> None:
    """
    Main method for app started as CLI
    """
    # parses the command-line arguments
    default_config_path = "ark-config.ini"
    parser = argparse.ArgumentParser(description="Convert between DSP resource IRIs and ARK URLs.")
    parser.add_argument("-c", "--config", help="config file (default {})".format(default_config_path))
    group = parser.add_mutually_exclusive_group()
    group.add_argument("-s", "--server", help="start server", action="store_true")
    group.add_argument("-i", "--iri", help="print the converted ARK URL from a given DSP resource IRI (add -v and -d optionally)")
    group.add_argument("-a", "--ark", help="print the converted DSP resource IRI (requires -r) or DSP URL from a given ARK ID")
    parser.add_argument("-r", "--resource", help="generate resource IRI", action="store_true")
    parser.add_argument("-v", "--value", help="value UUID (has to be provided with -i)")
    parser.add_argument("-d", "--date", help="DSP ARK timestamp (has to be provided with -i)")
    args = parser.parse_args()

    # reads the config and registry files
    if args.config is not None:
        config_path = args.config
    else:
        config_path = default_config_path

    try:
        settings = load_settings(config_path)

        if args.server:
            # starts the app as server
            server(settings)
        elif args.iri:
            # prints the converted ARK URL from a given DSP resource IRI
            print(ArkUrlFormatter(settings).resource_iri_to_ark_url(args.iri, args.value, args.date))
        elif args.ark:
            if args.resource:
                # prints the converted DSP resource IRI from a given ARK URL
                print(ArkUrlInfo(settings, args.ark).to_resource_iri())
            else:
                # prints the converted DSP URL from a given ARK URL
                print(ArkUrlInfo(settings, args.ark).to_redirect_url())
        else:
            parser.print_help()
    except ArkUrlException as ex:
        print(ex.message)
        sys.exit(1)
    except check_digit_py.CheckDigitException as ex:
        print(ex.message)
        sys.exit(1)


if __name__ == "__main__":
    main()
