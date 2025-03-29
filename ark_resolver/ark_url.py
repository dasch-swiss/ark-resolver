#!/usr/bin/env python3

# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

import base64
import re
import uuid
from string import Template
from urllib import parse

import ark_resolver.check_digit as check_digit_py


#################################################################################################
# Tools for generating and parsing DSP ARK URLs.

class ArkUrlSettings:
    """
    Settings used for the validation of values used in the context of ARK URLs
    """

    def __init__(self, config):
        self.config = config
        self.top_config = config["DEFAULT"]
        self.dsp_ark_version = 1
        self.project_id_pattern = "([0-9A-F]+)"
        self.uuid_pattern = "([A-Za-z0-9_=]+)"
        self.project_id_regex = re.compile("^" + self.project_id_pattern + "$")
        self.resource_iri_regex = re.compile("^http://rdfh.ch/" + self.project_id_pattern + "/([A-Za-z0-9_-]+)$")
        self.resource_int_id_factor = 982451653

        # Patterns for matching DSP ARK version 1 URLs.
        self.ark_path_pattern = "ark:/" + self.top_config[
            "ArkNaan"] + "/([0-9]+)(?:/" + self.project_id_pattern + "(?:/" + self.uuid_pattern + "(?:/" + self.uuid_pattern + r")?(?:\.([0-9]{8}T[0-9]{6,15}Z))?)?)?"
        self.ark_path_regex = re.compile("^" + self.ark_path_pattern + "$")
        self.ark_url_regex = re.compile(
            "^https?://" + self.top_config["ArkExternalHost"] + "/" + self.ark_path_pattern + "$")

        # Patterns for matching PHP-SALSAH ARK version 0 URLs.
        self.v0_ark_path_pattern = "ark:/" + self.top_config[
            "ArkNaan"] + r"/([0-9A-Fa-f]+)-([A-Za-z0-9]+)-[A-Za-z0-9]+(?:\.([0-9]{6,8}))?"
        self.v0_ark_path_regex = re.compile("^" + self.v0_ark_path_pattern + "$")
        self.v0_ark_url_regex = re.compile(
            "^https?://" + self.top_config["ArkExternalHost"] + "/" + self.v0_ark_path_pattern + "$")


class ArkUrlException(Exception):
    """
    Exception used in the context of ARK URLs
    """

    def __init__(self, message):
        self.message = message


class ArkUrlInfo:
    """
    Represents the information retrieved from a DSP ARK ID.
    """

    def __init__(self, settings, ark_id):
        self.settings = settings

        match = settings.ark_path_regex.match(ark_id)

        if match:
            # Yes. Is it a version 1 ARK ID?
            self.url_version = int(match.group(1))
        else:
            # No. Is it a version 0 ARK ID?
            match = settings.v0_ark_path_regex.match(ark_id)

            # If NOT None!, then it is a version 0 ARK ID.
            if match is not None:
                self.url_version = 0

        if match is None:
            raise ArkUrlException(f"Invalid ARK ID: {ark_id}")

        # Which version of ARK ID did we match?
        if self.url_version == settings.dsp_ark_version:
            # Version 1.
            self.project_id = match.group(2)
            escaped_resource_id_with_check_digit = match.group(3)

            if escaped_resource_id_with_check_digit is not None:
                self.resource_id = unescape_and_validate_uuid(
                    ark_url=ark_id,
                    escaped_uuid=escaped_resource_id_with_check_digit
                )

                escaped_value_id_with_check_digit = match.group(4)

                if escaped_value_id_with_check_digit is not None:
                    self.value_id = unescape_and_validate_uuid(
                        ark_url=ark_id,
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
                raise ArkUrlException(f"Invalid ARK ID (version 0 not allowed): {ark_id}")
        else:
            raise ArkUrlException(
                f"Invalid ARK ID {ark_id}. The version of the ARK ID doesn't match the version defined in the settings.")

        self.template_dict = {
            "url_version": self.url_version,
            "project_id": self.project_id,
            "resource_id": self.resource_id,
            "timestamp": self.timestamp
        }

    def to_redirect_url(self) -> str:
        """
        Checks if the object that it is called on is the top level object which is redirected to TopLevelObjectURL.
        If not, returns the redirect URL of either a PHP-SALSAH or DSP object.
        """
        if self.project_id is None:
            # return the redirect URL of the top level object
            return self.settings.top_config["TopLevelObjectUrl"]
        else:
            project_config = self.settings.config[self.project_id]

            if project_config.getboolean("UsePhp"):
                # return the redirect URL of a PHP-SALSAH object
                return self.to_php_redirect_url(project_config)
            else:
                # return the redirect URL of a DSP object
                return self.to_dsp_redirect_url(project_config)

    def to_resource_iri(self) -> str:
        """
        Converts an ARK URL to a DSP resource IRI. In case of an ARK URL version 0 the UUID for the IRI needs to be of
        version 5 and created from the DaSCH specific namespace and the resource_id coming from the ARK URL. This is for
        objects that have been migrated from salsah.org to DSP.
        """
        project_config = self.settings.config[self.project_id]
        resource_iri_template = Template(project_config["DSPResourceIri"])

        template_dict = self.template_dict.copy()
        template_dict["host"] = project_config["Host"]

        if self.url_version == 0:
            # in case of an ARK URL version 0, the resource_id generated from the salsah ID has to be converted to a
            # base64 UUID version 5
            generic_namespace_url = uuid.NAMESPACE_URL
            dasch_uuid_ns = uuid.uuid5(generic_namespace_url,
                                       "https://dasch.swiss")  # cace8b00-717e-50d5-bcb9-486f39d733a2
            resource_id = template_dict["resource_id"]
            dsp_iri = base64.urlsafe_b64encode(uuid.uuid5(dasch_uuid_ns, resource_id).bytes).decode("utf-8")
            # remove the padding ('==') from the end of the string
            dsp_iri = dsp_iri[:-2]
            template_dict["resource_id"] = dsp_iri

        return resource_iri_template.substitute(template_dict)

    def to_dsp_redirect_url(self, project_config) -> str:
        """
        In case it's called on a DSP object (either version 0 or version 1), converts an ARK URL to the URL that the
        client should be redirected to according to its type (project, resource, or value)
        """
        resource_iri_template = Template(project_config["DSPResourceIri"])
        project_iri_template = Template(project_config["DSPProjectIri"])

        template_dict = self.template_dict.copy()
        template_dict["host"] = project_config["Host"]

        # it's a project
        if self.resource_id is None:
            request_template = Template(project_config["DSPProjectRedirectUrl"])
            template_dict["project_host"] = project_config["ProjectHost"]
        # it's a resource
        elif self.value_id is None:
            if self.timestamp is None:
                request_template = Template(project_config["DSPResourceRedirectUrl"])
            else:
                request_template = Template(project_config["DSPResourceVersionRedirectUrl"])
        # it's a value
        elif self.value_id:
            template_dict["value_id"] = self.value_id
            if self.timestamp is None:
                request_template = Template(project_config["DSPValueRedirectUrl"])
            else:
                request_template = Template(project_config["DSPValueVersionRedirectUrl"])

        # in case of a version 0 ARK URL, convert the resource ID to a UUID (base64 encoded)
        if self.url_version == 0:
            res_iri = self.to_resource_iri()
            template_dict["resource_id"] = res_iri.split("/")[-1]

        # add the DSP resource IRI to the template_dict
        resource_iri = resource_iri_template.substitute(template_dict)
        url_encoded_resource_iri = parse.quote(resource_iri, safe="")
        template_dict["resource_iri"] = url_encoded_resource_iri

        # add the DSP project IRI to the template_dict
        project_iri = project_iri_template.substitute(template_dict)
        url_encoded_project_iri = parse.quote(project_iri, safe="")
        template_dict["project_iri"] = url_encoded_project_iri

        return request_template.substitute(template_dict)

    def to_php_redirect_url(self, project_config) -> str:
        """
        In case it's called on a PHP-SALSAH object, converts the ARK URL to the URL that the client should be
        redirected to.
        """
        template_dict = self.template_dict.copy()
        template_dict["host"] = project_config["Host"]

        # it's a resource
        if self.resource_id is not None:
            try:
                resource_int_id = (int(self.resource_id, 16) // self.settings.resource_int_id_factor) - 1
            except ValueError:
                raise ArkUrlException(f"Invalid resource ID: {self.resource_id}")

            template_dict["resource_int_id"] = resource_int_id

            if self.timestamp is None:
                request_template = Template(project_config["PhpResourceRedirectUrl"])
            else:
                request_template = Template(project_config["PhpResourceVersionRedirectUrl"])

                # The PHP server only takes timestamps in the format YYYYMMDD
                template_dict["timestamp"] = self.timestamp[0:8]

        # it's a project
        else:
            request_template = Template(project_config["DSPProjectRedirectUrl"])
            template_dict["project_host"] = project_config["ProjectHost"]

        return request_template.substitute(template_dict)


def add_check_digit_and_escape(uuid) -> str:
    """
    Adds a check digit to a Base64-encoded UUID, and escapes the result.
    """
    check_digit = check_digit_py.calculate_check_digit(uuid)
    uuid_with_check_digit = uuid + check_digit

    # Escape '-' as '=' in the resource ID and check digit, because '-' can be ignored in ARK URLs.
    return uuid_with_check_digit.replace('-', '=')


def unescape_and_validate_uuid(ark_url, escaped_uuid) -> str:
    """
    Unescapes a Base64-encoded UUID, validates its check digit, and returns the unescaped UUID without the check digit.
    """
    # '-' is escaped as '=' in the UUID and check digit, because '-' can be ignored in ARK URLs.
    unescaped_uuid = escaped_uuid.replace('=', '-')

    if not check_digit_py.is_valid(unescaped_uuid):
        raise ArkUrlException(f"Invalid ARK ID: {ark_url}")

    return unescaped_uuid[0:-1]


class ArkUrlFormatter:
    """
    Handles formatting of DSP resource IRIs into ARK URLs
    """

    def __init__(self, settings):
        self.settings = settings

    def resource_iri_to_ark_url(self, resource_iri, value_id=None, timestamp=None) -> str:
        """
        Converts a DSP resource IRI to an ARK URL.
        """
        # checks if given resource IRI is valid and matches (i.e. tokenizes) it into project_id and resource_id
        match = self.settings.resource_iri_regex.match(resource_iri)

        if match is None:
            raise ArkUrlException("Invalid resource IRI: {}".format(resource_iri))

        project_id = match.group(1)
        resource_id = match.group(2)
        escaped_resource_id_with_check_digit = add_check_digit_and_escape(resource_id)

        # checks if there is a value_id
        if value_id is not None:
            escaped_value_id_with_check_digit = add_check_digit_and_escape(value_id)
        else:
            escaped_value_id_with_check_digit = None

        # formats and returns the ARK URL
        return self.format_ark_url(
            project_id=project_id,
            resource_id_with_check_digit=escaped_resource_id_with_check_digit,
            value_id_with_check_digit=escaped_value_id_with_check_digit,
            timestamp=timestamp
        )

    def format_ark_url(self,
                       project_id,
                       resource_id_with_check_digit,
                       value_id_with_check_digit,
                       timestamp) -> str:
        """
        Formats and returns a DSP ARK URL from the given parameters and configuration.
        """
        if self.settings.top_config.getboolean("ArkHttpsProxy"):
            protocol = "https"
        else:
            protocol = "http"

        url = "{}://{}/ark:/{}/{}/{}/{}".format(
            protocol,
            self.settings.top_config["ArkExternalHost"],
            self.settings.top_config["ArkNaan"],
            self.settings.dsp_ark_version,
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
