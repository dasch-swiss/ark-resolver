#!/usr/bin/env python3

# Copyright @ 2015-2021 Data and Service Center for the Humanities (DaSCH)
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


import re
from urllib import parse
from string import Template

import base64url_check_digit


#################################################################################################
# Tools for generating and parsing Knora ARK URLs.

class ArkUrlSettings:
    def __init__(self, config):
        self.config = config
        self.top_config = config["DEFAULT"]
        self.knora_ark_version = 1
        self.project_id_pattern = "([0-9A-F]+)"
        self.uuid_pattern = "([A-Za-z0-9_=]+)"
        self.project_id_regex = re.compile("^" + self.project_id_pattern + "$")
        self.resource_iri_regex = re.compile("^http://rdfh.ch/" + self.project_id_pattern + "/([A-Za-z0-9_-]+)$")
        self.resource_int_id_factor = 982451653
        
        # Patterns for matching Knora ARK version 1 URLs.
        self.ark_path_pattern = "ark:/" + self.top_config["ArkNaan"] + "/([0-9]+)(?:/" + self.project_id_pattern + "(?:/" + self.uuid_pattern + "(?:/" + self.uuid_pattern + r")?(?:\.([0-9]{8}T[0-9]{6,15}Z))?)?)?"
        self.ark_path_regex = re.compile("^" + self.ark_path_pattern + "$")
        self.ark_url_regex = re.compile("^https?://" + self.top_config["ArkExternalHost"] + "/" + self.ark_path_pattern + "$")

        # Patterns for matching PHP-SALSAH ARK version 0 URLs.
        self.v0_ark_path_pattern = "ark:/" + self.top_config["ArkNaan"] + r"/([0-9A-Fa-f]+)-([A-Za-z0-9]+)-[A-Za-z0-9](?:\.([0-9]{6,8}))?"
        self.v0_ark_path_regex = re.compile("^" + self.v0_ark_path_pattern + "$")
        self.v0_ark_url_regex = re.compile("^https?://" + self.top_config["ArkExternalHost"] + "/" + self.v0_ark_path_pattern + "$")


class ArkUrlException(Exception):
    def __init__(self, message):
        self.message = message


# Represents the information retrieved from a Knora ARK URL.
class ArkUrlInfo:
    def __init__(self, settings, ark_url, path_only=False):
        self.settings = settings

        # Are we matching just the path part of the URL?
        if path_only:
            # Yes. Is it a version 1 ARK ID?
            match = settings.ark_path_regex.match(ark_url)

            if match is not None:
                # Yes.
                self.url_version = int(match.group(1))
            else:
                # No. Is it a version 0 ARK ID?
                match = settings.v0_ark_path_regex.match(ark_url)

                if match is not None:
                    self.url_version = 0

        else:
            # We are matching a whole URL. Does it contain a version 1 ARK ID?
            match = settings.ark_url_regex.match(ark_url)

            if match is not None:
                # Yes.
                self.url_version = int(match.group(1))
            else:
                # No. Does it contain a version 0 ARK ID?
                match = settings.v0_ark_url_regex.match(ark_url)

                if match is not None:
                    self.url_version = 0

        if match is None:
            raise ArkUrlException("Invalid ARK ID: {}".format(ark_url))

        # Which version of ARK ID did we match?
        if self.url_version == settings.knora_ark_version:
            # Version 1.
            self.project_id = match.group(2)
            escaped_resource_id_with_check_digit = match.group(3)

            if escaped_resource_id_with_check_digit is not None:
                self.resource_id = unescape_and_validate_uuid(
                    ark_url=ark_url,
                    escaped_uuid=escaped_resource_id_with_check_digit
                )

                escaped_value_id_with_check_digit = match.group(4)

                if escaped_value_id_with_check_digit is not None:
                    self.value_id = unescape_and_validate_uuid(
                        ark_url=ark_url,
                        escaped_uuid=escaped_value_id_with_check_digit
                    )
                else:
                    self.value_id = None

                self.timestamp = match.group(5)
            else:
                self.resource_id = None
                self.value_id = None
                self.timestamp = None
        elif self.url_version == 0:
            # Version 0.
            self.project_id = match.group(1).upper()
            self.resource_id = match.group(2)
            self.value_id = None

            submitted_timestamp = match.group(3)

            if submitted_timestamp is None or len(submitted_timestamp) < 8:
                self.timestamp = None
            else:
                self.timestamp = submitted_timestamp

            project_config = self.settings.config[self.project_id]

            if not project_config.getboolean("AllowVersion0"):
                raise ArkUrlException("Invalid ARK ID (version 0 not allowed): {}".format(ark_url))
        else:
            raise ArkUrlException("Invalid ARK ID: {}".format(ark_url))

        self.template_dict = {
            "url_version": self.url_version,
            "project_id": self.project_id,
            "resource_id": self.resource_id,
            "timestamp": self.timestamp
        }

    # Converts an ARK URL to the URL that the client should be redirected to.
    def to_redirect_url(self):
        if self.project_id is None:
            return self.settings.top_config["TopLevelObjectUrl"]
        else:
            project_config = self.settings.config[self.project_id]

            if project_config.getboolean("UsePhp"):
                return self.to_php_redirect_url(project_config)
            else:
                return self.to_knora_redirect_url(project_config)

    def to_resource_iri(self):
        project_config = self.settings.config[self.project_id]
        resource_iri_template = Template(project_config["KnoraResourceIri"])

        template_dict = self.template_dict.copy()
        template_dict["host"] = project_config["Host"]

        return resource_iri_template.substitute(template_dict)

    def to_knora_redirect_url(self, project_config):
        resource_iri_template = Template(project_config["KnoraResourceIri"])
        project_iri_template = Template(project_config["KnoraProjectIri"])

        if self.resource_id is None:
            request_template = Template(project_config["KnoraProjectRedirectUrl"])
        elif self.value_id is None:
            if self.timestamp is None:
                request_template = Template(project_config["KnoraResourceRedirectUrl"])
            else:
                request_template = Template(project_config["KnoraResourceVersionRedirectUrl"])
        elif self.timestamp is None:
            request_template = Template(project_config["KnoraValueRedirectUrl"])
        else:
            request_template = Template(project_config["KnoraValueVersionRedirectUrl"])

        template_dict = self.template_dict.copy()
        template_dict["host"] = project_config["Host"]

        resource_iri = resource_iri_template.substitute(template_dict)
        url_encoded_resource_iri = parse.quote(resource_iri, safe="")
        template_dict["resource_iri"] = url_encoded_resource_iri

        project_iri = project_iri_template.substitute(template_dict)
        url_encoded_project_iri = parse.quote(project_iri, safe="")
        template_dict["project_iri"] = url_encoded_project_iri

        if self.value_id is not None:
            template_dict["value_id"] = self.value_id

        return request_template.substitute(template_dict)

    def to_php_redirect_url(self, project_config):
        template_dict = self.template_dict.copy()

        template_dict["host"] = project_config["Host"]

        if self.resource_id is not None:
            resource_int_id = (int(self.resource_id, 16) // self.settings.resource_int_id_factor) - 1
            template_dict["resource_int_id"] = resource_int_id

            if self.timestamp is None:
                request_template = Template(project_config["PhpResourceRedirectUrl"])
            else:
                request_template = Template(project_config["PhpResourceVersionRedirectUrl"])

                # The PHP server only takes timestamps in the format YYYYMMDD
                template_dict["timestamp"] = self.timestamp[0:8]
        else:
            request_template = Template(project_config["KnoraProjectRedirectUrl"])

        return request_template.substitute(template_dict)


# Adds a check digit to a Base64-encoded UUID, and escapes the result.
def add_check_digit_and_escape(uuid):
    check_digit = base64url_check_digit.calculate_check_digit(uuid)
    uuid_with_check_digit = uuid + check_digit

    # Escape '-' as '=' in the resource ID and check digit, because '-' can be ignored in ARK URLs.
    return uuid_with_check_digit.replace('-', '=')


# Unescapes a Base64-encoded UUID, validates its check digit, and returns the unescaped UUID
# without the check digit.
def unescape_and_validate_uuid(ark_url, escaped_uuid):
    # '-' is escaped as '=' in the UUID and check digit, because '-' can be ignored in ARK URLs.
    unescaped_uuid = escaped_uuid.replace('=', '-')

    if not base64url_check_digit.is_valid(unescaped_uuid):
        raise ArkUrlException("Invalid ARK ID: {}".format(ark_url))

    return unescaped_uuid[0:-1]


# Formats ARK URLs.
class ArkUrlFormatter:
    def __init__(self, settings):
        self.settings = settings

    # Converts a Knora resource IRI to an ARK URL.
    def resource_iri_to_ark_url(self, resource_iri, value_id=None, timestamp=None):
        match = self.settings.resource_iri_regex.match(resource_iri)

        if match is None:
            raise ArkUrlException("Invalid resource IRI: {}".format(resource_iri))

        project_id = match.group(1)
        resource_id = match.group(2)
        escaped_resource_id_with_check_digit = add_check_digit_and_escape(resource_id)

        if value_id is not None:
            escaped_value_id_with_check_digit = add_check_digit_and_escape(value_id)
        else:
            escaped_value_id_with_check_digit = None

        return self.format_ark_url(
            project_id=project_id,
            resource_id_with_check_digit=escaped_resource_id_with_check_digit,
            value_id_with_check_digit=escaped_value_id_with_check_digit,
            timestamp=timestamp
        )

    # Converts information about a PHP resource to an ARK URL.
    def php_resource_to_ark_url(self, php_resource_id, project_id, timestamp=None):
        knora_resource_id = format((php_resource_id + 1) * self.settings.resource_int_id_factor, 'x')
        check_digit = base64url_check_digit.calculate_check_digit(knora_resource_id)
        resource_id_with_check_digit = knora_resource_id + check_digit

        return self.format_ark_url(
            project_id=project_id,
            resource_id_with_check_digit=resource_id_with_check_digit,
            value_id_with_check_digit=None,
            timestamp=timestamp
        )

    # Formats a Knora ARK URL.
    def format_ark_url(self,
                       project_id,
                       resource_id_with_check_digit,
                       value_id_with_check_digit,
                       timestamp):
        if self.settings.top_config.getboolean("ArkHttpsProxy"):
            protocol = "https"
        else:
            protocol = "http"

        url = "{}://{}/ark:/{}/{}/{}/{}".format(
            protocol,
            self.settings.top_config["ArkExternalHost"],
            self.settings.top_config["ArkNaan"],
            self.settings.knora_ark_version,
            project_id,
            resource_id_with_check_digit
        )

        # If there's a value UUID, add it.
        if value_id_with_check_digit is not None:
            url += "/" + value_id_with_check_digit

        # If there's a timestamp, add it as an object variant.
        if timestamp is not None:
            url += "." + timestamp

        return url

    def format_resource_iri(self, php_resource_id, project_id):
        knora_resource_id = format((php_resource_id + 1) * self.settings.resource_int_id_factor, 'x')
        project_config = self.settings.config[project_id]
        resource_iri_template = Template(project_config["KnoraResourceIri"])

        template_dict = {
            "project_id": project_id,
            "resource_id": knora_resource_id
        }

        return resource_iri_template.substitute(template_dict)
