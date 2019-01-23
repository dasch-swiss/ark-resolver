#!/usr/bin/env python3

# Copyright @ 2015-2019 the contributors (see Contributors.md).
#
# This file is part of Knora.
#
# Knora is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Knora is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public
# License along with Knora.  If not, see <http://www.gnu.org/licenses/>.


#################################################################################################
# Knora ARK redirect server and conversion utility.
#
# For help on command-line options, run with --help.
#################################################################################################


import argparse
import configparser
import traceback
import os

from sanic import Sanic, response

import base64url_check_digit
from ark_url import ArkUrlInfo, ArkUrlFormatter, ArkUrlException, ArkUrlSettings


#################################################################################################
# Server implementation.

app = Sanic()


@app.get('/<path:path>')
async def catch_all(_, path=""):
    try:
        redirect_url = ArkUrlInfo(settings=app.config.settings, ark_url=path, path_only=True).to_redirect_url()
    except ArkUrlException as ex:
        return response.text(
            body=ex.message,
            status=400
        )
    except base64url_check_digit.CheckDigitException as ex:
        return response.text(
            body=ex.message,
            status=400
        )
    except KeyError:
        return response.text(
            body="Invalid ARK ID",
            status=400
        )

    return response.redirect(redirect_url)


def server(settings):
    app.config.settings = settings
    app.run(host=settings.top_config["ArkInternalHost"], port=settings.top_config.getint("ArkInternalPort"))


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
    ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri)
    assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
    print("OK")

    print("generate an ARK URL for a resource IRI with a timestamp: ", end='')
    ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, timestamp="20190118T102919000031660Z")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20190118T102919000031660Z"
    print("OK")

    print("generate an ARK URL for a PHP resource without a timestamp: ", end='')
    ark_url = ark_url_formatter.php_resource_to_ark_url(php_resource_id=1, project_id="0803")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0803/751e0b8am"
    print("OK")

    print("generate an ARK URL for a PHP resource with a timestamp: ", end='')
    ark_url = ark_url_formatter.php_resource_to_ark_url(php_resource_id=1, project_id="0803", timestamp="20190118T102919000031660Z")
    assert ark_url == "https://ark.example.org/ark:/00000/1/0803/751e0b8am.20190118T102919000031660Z"
    print("OK")

    print("parse an ARK URL representing the top-level object: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://dasch.swiss"
    print("OK")

    print("parse an ARK project URL: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/admin/projects/http%3A%2F%2Frdfh.ch%2Fprojects%2F0001"
    print("OK")

    print("parse an ARK URL for a Knora resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/v2/resources/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA"
    print("OK")

    print("parse an ARK HTTP URL for a Knora resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "http://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/v2/resources/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA"
    print("OK")

    print("parse an ARK URL for a Knora resource with a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20190118T102919000031660Z")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://0.0.0.0:3333/v2/resources/http%3A%2F%2Frdfh.ch%2F0001%2Fcmfk1DMHRBiR4-_6HXpEFA?version=20190118T102919000031660Z"
    print("OK")

    print("parse an ARK URL for a PHP resource without a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0803/751e0b8am")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/1"
    print("OK")

    print("parse an ARK URL for a PHP resource with a timestamp: ", end='')
    ark_url_info = ArkUrlInfo(settings, "https://ark.example.org/ark:/00000/1/0803/751e0b8am.20190118T102919000031660Z")
    redirect_url = ark_url_info.to_redirect_url()
    assert redirect_url == "http://data.dasch.swiss/resources/1?citdate=20190118"
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
    # Default configuration filename.
    default_config_filename = "ark-config.ini"
    default_registry_filename = "ark-registry.ini"

    # Default configuration from environment variables.
    environment_vars = {
        "ArkExternalHost": os.environ.get("ARK_EXTERNAL_HOST", "ark.example.org"),
        "ArkInternalHost": os.environ.get("ARK_INTERNAL_HOST", "0.0.0.0"),
        "ArkInternalPort": os.environ.get("ARK_INTERNAL_PORT", "3336"),
        "ArkNaan": os.environ.get("ARK_NAAN", "00000"),
        "ArkHttpsProxy":  os.environ.get("ARK_HTTPS_PROXY", "true")
    }

    # Parse command-line arguments.
    parser = argparse.ArgumentParser(description="Convert between Knora resource IRIs and ARK URLs.")
    parser.add_argument("-c", "--config", help="config file (default {})".format(default_config_filename))
    parser.add_argument("-r", "--registry", help="registry file (default {})".format(default_registry_filename))
    group = parser.add_mutually_exclusive_group()
    group.add_argument("-s", "--server", help="start server", action="store_true")
    group.add_argument("-a", "--ark", help="ARK URL")
    group.add_argument("-i", "--iri", help="resource IRI")
    group.add_argument("-n", "--number", help="resource number for PHP server")
    group.add_argument("-t", "--test", help="run tests", action="store_true")
    parser.add_argument("-d", "--date", help="Knora ARK timestamp (with -i or -p)")
    parser.add_argument("-p", "--project", help="project ID (with -n)")
    args = parser.parse_args()

    # Read the config and registry files.

    config = configparser.ConfigParser(defaults=environment_vars)

    try:
        if args.config is not None:
            config.read_file(open(args.config))
        else:
            config.read_file(open(default_config_filename))

        if args.registry is not None:
            config.read_file(open(args.registry))
        else:
            config.read_file(open(default_registry_filename))

        settings = ArkUrlSettings(config)

        if args.server:
            server(settings)
        elif args.test:
            try:
                test(settings)
            except Exception:
                traceback.print_exc()
                exit(1)
        elif args.iri:
            print(ArkUrlFormatter(settings).resource_iri_to_ark_url(args.iri, args.date))
        elif args.number:
            print(ArkUrlFormatter(settings).php_resource_to_ark_url(int(args.number), args.project, args.date))
        elif args.ark:
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
