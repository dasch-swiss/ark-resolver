#!/usr/bin/env python3

# Copyright @ 2015-2021 Data and Service Center for the Humanities (DaSCH)
#
# This file is part of DSP.
#
# DSP is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# DSP is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public
# License along with DSP.  If not, see <http://www.gnu.org/licenses/>.


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
from io import StringIO

import requests
from sanic import Sanic, response
from sanic.log import logger
from sanic_cors import CORS

import base64url_check_digit
from ark_url import ArkUrlInfo, ArkUrlFormatter, ArkUrlException, ArkUrlSettings

#################################################################################################
# Server implementation.

app = Sanic('ark_resolver')
CORS(app)


@app.get("/make_php_ark_url")
async def make_php_ark_url(req):
    project_id = req.args["project_id"][0]

    if app.config.settings.project_id_regex.match(project_id) is None:
        return response.text("Invalid project ID: {}".format(project_id), status=400)

    php_resource_id = int(req.args["resource_id"][0])
    ark_url = ArkUrlFormatter(app.config.settings).php_resource_to_ark_url(php_resource_id, project_id)
    return response.text(ark_url)


def get_config():
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
    return config_output.getvalue()


@app.get("/config")
async def config_get(_):
    return response.text(get_config())


@app.head("/config")
async def config_head(_):
    config_str = get_config()

    headers = {
        "Content-Length": str(len(config_str)),
        "Content-Type": "text/plain; charset=utf-8"
    }

    return response.text("", headers=headers)


@app.post("/reload")
async def reload(req):
    # Get the signature submitted with the request.

    if "X-Hub-Signature" not in req.headers:
        return response.text("Unauthorized", status=401)

    signature_header = req.headers["X-Hub-Signature"]

    if not signature_header.startswith("sha1="):
        return response.text("Unauthorized", status=401)

    submitted_signature = signature_header.split('=')[1]

    # Compute a signature for the request using the configured GitHub secret.
    secret = app.config.settings.top_config["ArkGitHubSecret"]
    computed_signature = hmac.new(secret.encode(), req.body, hashlib.sha1).hexdigest()

    # If the submitted signature is the same as the computed one, the request is valid.
    if hmac.compare_digest(submitted_signature, computed_signature):
        # Reload configuration.
        settings = load_settings(app.config.config_path)
        app.config.settings = settings
        logger.info("Configuration reloaded.")
        return response.text("", status=204)
    else:
        return response.text("Unauthorized", status=401)


@app.get('/<path:path>')
async def catch_all(_, path=""):
    try:
        redirect_url = ArkUrlInfo(settings=app.config.settings, ark_url=path, path_only=True).to_redirect_url()
    except ArkUrlException as ex:
        return response.text(body=ex.message, status=400)

    except base64url_check_digit.CheckDigitException as ex:
        return response.text(body=ex.message, status=400)

    except KeyError:
        return response.text(body="Invalid ARK ID", status=400)

    return response.redirect(redirect_url)


def server(settings):
    app.config.settings = settings
    app.run(host=settings.top_config["ArkInternalHost"], port=settings.top_config.getint("ArkInternalPort"))


#################################################################################################
# Loading of config and registry files.

# Loads configuration and returns an ArkUrlSettings.
def load_settings(config_path):
    app.config.config_path = config_path

    # Default configuration from environment variables.
    environment_vars = {
        "ArkExternalHost": os.environ.get("ARK_EXTERNAL_HOST", "ark.example.org"),
        "ArkInternalHost": os.environ.get("ARK_INTERNAL_HOST", "0.0.0.0"),
        "ArkInternalPort": os.environ.get("ARK_INTERNAL_PORT", "3336"),
        "ArkNaan": os.environ.get("ARK_NAAN", "00000"),
        "ArkHttpsProxy": os.environ.get("ARK_HTTPS_PROXY", "true"),
        "ArkRegistry": os.environ.get("ARK_REGISTRY", "ark-registry.ini"),
        "ArkGitHubSecret": os.environ.get("ARK_GITHUB_SECRET", "")
    }

    # Read the config and registry files.

    config = configparser.ConfigParser(defaults=environment_vars)
    config.read_file(open(config_path))

    registry_path = config["DEFAULT"]["ArkRegistry"]

    if registry_path.startswith("http"):
        registry_str = requests.get(registry_path).text
        config.read_string(registry_str, source=registry_path)
    else:
        config.read_file(open(registry_path))

    settings = ArkUrlSettings(config)

    return settings


#################################################################################################
# Command-line invocation.

def main():
    # Parse command-line arguments.
    default_config_path = "ark-config.ini"
    parser = argparse.ArgumentParser(description="Convert between DSP resource IRIs and ARK URLs.")
    parser.add_argument("-c", "--config", help="config file (default {})".format(default_config_path))
    group = parser.add_mutually_exclusive_group()
    group.add_argument("-s", "--server", help="start server", action="store_true")
    group.add_argument("-a", "--ark", help="ARK URL")
    group.add_argument("-i", "--iri", help="resource IRI")
    group.add_argument("-n", "--number", help="resource number for PHP server")
    parser.add_argument("-r", "--resource", help="generate resource IRI", action="store_true")
    parser.add_argument("-v", "--value", help="value UUID (with -i)")
    parser.add_argument("-d", "--date", help="DSP ARK timestamp (with -i or -n)")
    parser.add_argument("-p", "--project", help="project ID (with -n)")
    args = parser.parse_args()

    # Read the config and registry files.

    if args.config is not None:
        config_path = args.config
    else:
        config_path = default_config_path

    try:
        settings = load_settings(config_path)

        if args.server:
            server(settings)
        elif args.iri:
            print(ArkUrlFormatter(settings).resource_iri_to_ark_url(args.iri, args.value, args.date))
        elif args.number:
            if args.project is None:
                raise ArkUrlException("Project ID is required with resource number")
            elif args.resource:
                print(ArkUrlFormatter(settings).format_resource_iri(int(args.number), args.project))
            else:
                print(ArkUrlFormatter(settings).php_resource_to_ark_url(int(args.number), args.project))
        elif args.ark:
            if args.resource:
                print(ArkUrlInfo(settings, args.ark).to_resource_iri())
            else:
                print(ArkUrlInfo(settings, args.ark).to_redirect_url())
        else:
            parser.print_help()
    except ArkUrlException as ex:
        print(ex.message)
        exit(1)
    except base64url_check_digit.CheckDigitException as ex:
        print(ex.message)
        exit(1)


if __name__ == "__main__":
    main()
