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
import traceback
import os
from io import StringIO
import hashlib
import hmac

from sanic import Sanic, response
from sanic.response import text
from sanic.log import logger
from sanic_cors import CORS, cross_origin
import requests

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
        "ArkHttpsProxy":  os.environ.get("ARK_HTTPS_PROXY", "true"),
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
# Automated tests.

def test(settings):
    ark_url_formatter = ArkUrlFormatter(settings)
    correct_resource_id = "cmfk1DMHRBiR4-_6HXpEFA"

    print("reject a string without a check digit: ", end='')
    assert not base64url_check_digit.is_valid(correct_resource_id)
    print("OK")

    print("calculate a check digit for a string and validate it: ", end='')
    correct_resource_id_check_digit = "n"
    check_digit = base64url_check_digit.calculate_check_digit(correct_resource_id)
    assert check_digit == correct_resource_id_check_digit
    correct_resource_id_with_correct_check_digit = correct_resource_id + check_digit
    assert base64url_check_digit.is_valid(correct_resource_id_with_correct_check_digit)
    print("OK")

    print("reject a string with an incorrect check digit: ", end='')
    correct_resource_id_with_incorrect_check_digit = correct_resource_id + "m"
    assert not base64url_check_digit.is_valid(correct_resource_id_with_incorrect_check_digit)
    print("OK")

    print("reject a string with a missing character: ", end='')
    resource_id_with_missing_character = "cmfk1DMHRBiR4-6HXpEFA"
    resource_id_with_missing_character_and_correct_check_digit = resource_id_with_missing_character + correct_resource_id_check_digit
    assert not base64url_check_digit.is_valid(resource_id_with_missing_character_and_correct_check_digit)
    print("OK")

    print("reject a string with an incorrect character: ", end='')
    resource_id_with_incorrect_character = "cmfk1DMHRBir4-_6HXpEFA"
    resource_id_with_incorrect_character_and_correct_check_digit = resource_id_with_incorrect_character + correct_resource_id_check_digit
    assert not base64url_check_digit.is_valid(resource_id_with_incorrect_character_and_correct_check_digit)
    print("OK")

    print("reject a string with swapped characters: ", end='')
    resource_id_with_swapped_characters = "cmfk1DMHRBiR4_-6HXpEFA"
    resource_id_with_swapped_characters_and_correct_check_digit = resource_id_with_swapped_characters + correct_resource_id_check_digit
    assert not base64url_check_digit.is_valid(resource_id_with_swapped_characters_and_correct_check_digit)
    print("OK")

    print("generate an ARK URL for a resource IRI without a timestamp: ", end='')
    resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA"
    ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri)
    assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
    print("OK")

    print("generate an ARK URL for a resource IRI with a timestamp: ", end='')
    ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, timestamp="20180604T085622513Z")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z"
    print("OK")

    print("generate an ARK URL for a resource IRI and value UUID without a timestamp: ", end='')
    value_id = "pLlW4ODASumZfZFbJdpw1g"
    ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, value_id=value_id)
    assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu"
    print("OK")

    print("generate an ARK URL for a resource IRI and value UUID with a timestamp: ", end='')
    ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, value_id=value_id, timestamp="20180604T085622513Z")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622513Z"
    print("OK")

    print("generate a version 1 ARK URL for a PHP resource without a timestamp: ", end='')
    ark_url = ark_url_formatter.php_resource_to_ark_url(php_resource_id=1, project_id="0803")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0803/751e0b8am"
    print("OK")

    print("generate a version 1 ARK URL for a PHP resource with a timestamp: ", end='')
    ark_url = ark_url_formatter.php_resource_to_ark_url(php_resource_id=1, project_id="0803", timestamp="20180604T085622513Z")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0803/751e0b8am.20180604T085622513Z"
    print("OK")

    print("parse an ARK URL representing the top-level object: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://dasch.swiss"
    print("OK")

    print("parse an ARK project URL: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/project/0001/info"
    print("OK")

    print("parse an ARK URL for a DSP resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/resource/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA"
    print("OK")

    print("parse an ARK HTTP URL for a DSP resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "http://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/resource/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA"
    print("OK")

    print("parse an ARK URL for a DSP resource with a timestamp with a fractional part: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/resource/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA?version=20180604T085622513Z"
    print("OK")

    print("parse an ARK URL for a DSP resource with a timestamp without a fractional part: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/resource/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA?version=20180604T085622Z"
    print("OK")

    print("parse an ARK URL for a DSP resource and value UUID without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/value/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA/pLlW4ODASumZfZFbJdpw1g"
    print("OK")

    print("parse an ARK URL for a DSP resource and value UUID with a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/value/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA/pLlW4ODASumZfZFbJdpw1g?version=20180604T085622Z"
    print("OK")

    print("parse a version 1 ARK URL for a PHP resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0803/751e0b8am")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/1"
    print("OK")

    print("parse an ARK URL for a PHP resource with a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0803/751e0b8am.20190118T102919Z")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/1?citdate=20190118"
    print("OK")

    print("parse a version 0 ARK URL for a PHP resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "http://ark.example.org/ark:/00000/080e-76bb2132d30d6-0")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/2126045"
    print("OK")

    print("parse a version 0 ARK URL for a PHP resource with a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "http://ark.example.org/ark:/00000/080e-76bb2132d30d6-0.20190129")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/2126045?citdate=20190129"
    print("OK")

    print("parse a version 0 ARK URL for a PHP resource with a timestamp that's too short: ", end='')
    ark_url_info = ArkUrlInfo(settings, "http://ark.example.org/ark:/00000/080e-76bb2132d30d6-0.2019111")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/2126045"
    print("OK")

    print("convert a version 0 ARK URL to a custom resource IRI, and then to a DSP-API redirect URL: ", end='')
    ark_url_info = ArkUrlInfo(settings, "http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519")
    resource_iri = ark_url_info.to_resource_iri()
    assert resource_iri == "http://rdfh.ch/0002/751e0b8a"
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resource/http%3A%2F%2Frdfh.ch%2F0002%2F751e0b8a"
    print("OK")

    print("convert a PHP resource ID to the same custom resource IRI, and then to the same DSP-API redirect URL:", end='')
    resource_iri = ArkUrlFormatter(settings).format_resource_iri(1, "0002")
    assert resource_iri == "http://rdfh.ch/0002/751e0b8a"
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resource/http%3A%2F%2Frdfh.ch%2F0002%2F751e0b8a"
    print("OK")

    print("reject an ARK URL that doesn't pass check digit validation: ", end='')
    rejected = False

    try:
        ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBir4=_6HXpEFAn")
    except ArkUrlException:
        rejected = True

    assert rejected
    print("OK")


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
    group.add_argument("-t", "--test", help="run tests", action="store_true")
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
        elif args.test:
            try:
                test(settings)
            except Exception:
                traceback.print_exc()
                exit(1)
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
