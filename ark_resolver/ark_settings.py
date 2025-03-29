from __future__ import annotations

import configparser
import os
import re
from configparser import ConfigParser
from configparser import SectionProxy
from dataclasses import dataclass

import requests


@dataclass
class ArkUrlSettings:
    """
    Settings used for the validation of values used in the context of ARK URLs
    """

    # TODO: get rid of these config-parser types
    config: ConfigParser
    top_config: SectionProxy
    dsp_ark_version: int
    ark_path_regex: re.Pattern
    v0_ark_path_regex: re.Pattern
    resource_iri_regex: re.Pattern
    resource_int_id_factor: int = 982451653

    @staticmethod
    def from_config(config: ConfigParser) -> ArkUrlSettings:
        top_config = config["DEFAULT"]
        dsp_ark_version = 1
        project_id_pattern = "([0-9A-F]+)"
        uuid_pattern = "([A-Za-z0-9_=]+)"
        resource_iri_regex = re.compile("^http://rdfh.ch/" + project_id_pattern + "/([A-Za-z0-9_-]+)$")
        resource_int_id_factor = 982451653

        # Patterns for matching DSP ARK version 1 URLs.
        ark_path_pattern = (
            "ark:/"
            + top_config["ArkNaan"]
            + "/([0-9]+)(?:/"
            + project_id_pattern
            + "(?:/"
            + uuid_pattern
            + "(?:/"
            + uuid_pattern
            + r")?(?:\.([0-9]{8}T[0-9]{6,15}Z))?)?)?"
        )
        ark_path_regex = re.compile("^" + ark_path_pattern + "$")

        # Patterns for matching PHP-SALSAH ARK version 0 URLs.
        v0_ark_path_pattern = "ark:/" + top_config["ArkNaan"] + r"/([0-9A-Fa-f]+)-([A-Za-z0-9]+)-[A-Za-z0-9]+(?:\.([0-9]{6,8}))?"
        v0_ark_path_regex = re.compile("^" + v0_ark_path_pattern + "$")

        return ArkUrlSettings(
            config=config,
            top_config=top_config,
            dsp_ark_version=dsp_ark_version,
            ark_path_regex=ark_path_regex,
            v0_ark_path_regex=v0_ark_path_regex,
            resource_iri_regex=resource_iri_regex,
            resource_int_id_factor=resource_int_id_factor,
        )


def load_settings(config_path: str) -> ArkUrlSettings:
    """
    Loads configuration from given path and returns an ArkUrlSettings.
    """
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

    settings = ArkUrlSettings.from_config(config)

    return settings
