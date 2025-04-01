import configparser
import os
import re
from configparser import ConfigParser

import requests


class ArkUrlSettings:
    """
    Settings used for the validation of values used in the context of ARK URLs
    """

    def __init__(self, config: ConfigParser) -> None:
        self.config = config
        self.top_config = config["DEFAULT"]
        self.dsp_ark_version = 1
        self.project_id_pattern = f"([0-9A-Fa-f]{4})"
        self.uuid_pattern = "([A-Za-z0-9_=]+)"
        self.project_id_regex = re.compile("^" + self.project_id_pattern + "$")
        self.resource_iri_regex = re.compile("^http://rdfh.ch/" + self.project_id_pattern + "/([A-Za-z0-9_-]+)$")
        self.resource_int_id_factor = 982451653

        # Patterns for matching DSP ARK version 1 URLs.
        self.ark_path_pattern = (
            "ark:/"
            + self.top_config["ArkNaan"]
            + "/([0-9]+)(?:/"
            + self.project_id_pattern
            + "(?:/"
            + self.uuid_pattern
            + "(?:/"
            + self.uuid_pattern
            + r")?(?:\.([0-9]{8}T[0-9]{6,15}Z))?)?)?"
        )
        self.ark_path_regex = re.compile("^" + self.ark_path_pattern + "$")
        self.ark_url_regex = re.compile("^https?://" + self.top_config["ArkExternalHost"] + "/" + self.ark_path_pattern + "$")

        # Patterns for matching PHP-SALSAH ARK version 0 URLs.
        self.v0_ark_path_pattern = (
            "ark:/" + self.top_config["ArkNaan"] + r"/([0-9A-Fa-f]{4})-([A-Za-z0-9]+)-[A-Za-z0-9]+(?:\.([0-9]{6,8}))?"
        )
        self.v0_ark_path_regex = re.compile("^" + self.v0_ark_path_pattern + "$")
        self.v0_ark_url_regex = re.compile("^https?://" + self.top_config["ArkExternalHost"] + "/" + self.v0_ark_path_pattern + "$")


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

    settings = ArkUrlSettings(config)

    return settings
