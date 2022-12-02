import os
import pickle
import unittest

from src import base64url_check_digit
from src.ark_url import ArkUrlFormatter, ArkUrlInfo, ArkUrlException


class TestArkResolver(unittest.TestCase):

    settings = None

    @classmethod
    def setUpClass(cls):
        """
        loads the settings from a pickle file
        """
        try:
            with open(os.path.join('test', 'settings.pkl'), 'rb') as mock_settings:
                cls.settings = pickle.load(mock_settings)
        except FileNotFoundError:
            # when starting test directly from the IDE, it is already inside 'test' folder
            with open(os.path.join('settings.pkl'), 'rb') as mock_settings:
                cls.settings = pickle.load(mock_settings)

    def test_base64url_check_digit(self):
        correct_resource_id = "cmfk1DMHRBiR4-_6HXpEFA"

        # reject a string without a check digit
        assert not base64url_check_digit.is_valid(correct_resource_id)

        # calculate a check digit for a string and validate it
        correct_resource_id_check_digit = "n"
        check_digit = base64url_check_digit.calculate_check_digit(correct_resource_id)
        assert check_digit == correct_resource_id_check_digit
        correct_resource_id_with_correct_check_digit = correct_resource_id + check_digit
        assert base64url_check_digit.is_valid(correct_resource_id_with_correct_check_digit)

        # reject a string with an incorrect check digit
        correct_resource_id_with_incorrect_check_digit = correct_resource_id + "m"
        assert not base64url_check_digit.is_valid(correct_resource_id_with_incorrect_check_digit)

        # reject a string with a missing character
        resource_id_with_missing_character = "cmfk1DMHRBiR4-6HXpEFA"
        resource_id_with_missing_character_and_correct_check_digit = resource_id_with_missing_character + correct_resource_id_check_digit
        assert not base64url_check_digit.is_valid(resource_id_with_missing_character_and_correct_check_digit)

        # reject a string with an incorrect character
        resource_id_with_incorrect_character = "cmfk1DMHRBir4-_6HXpEFA"
        resource_id_with_incorrect_character_and_correct_check_digit = resource_id_with_incorrect_character + correct_resource_id_check_digit
        assert not base64url_check_digit.is_valid(resource_id_with_incorrect_character_and_correct_check_digit)

        # reject a string with swapped characters
        resource_id_with_swapped_characters = "cmfk1DMHRBiR4_-6HXpEFA"
        resource_id_with_swapped_characters_and_correct_check_digit = resource_id_with_swapped_characters + correct_resource_id_check_digit
        assert not base64url_check_digit.is_valid(resource_id_with_swapped_characters_and_correct_check_digit)

    def test_ark_url_formatter(self):
        ark_url_formatter = ArkUrlFormatter(self.settings)
        # generate an ARK URL from a resource IRI without a timestamp
        resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA"
        ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri)
        assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"

        # generate an ARK URL from a resource IRI with a timestamp
        ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, timestamp="20180604T085622513Z")
        assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z"

        # generate an ARK URL from a resource IRI and value UUID without a timestamp
        value_id = "pLlW4ODASumZfZFbJdpw1g"
        ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, value_id=value_id)
        assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu"

        # generate an ARK URL from a resource IRI and value UUID with a timestamp
        ark_url = ark_url_formatter.resource_iri_to_ark_url(resource_iri=resource_iri, value_id=value_id,
                                                            timestamp="20180604T085622513Z")
        assert ark_url == "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622513Z"

    def test_ark_url_info_redirect_top_level_object(self):
        # parse and redirect an ARK URL representing the top-level object
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://dasch.swiss"

    def test_ark_url_info_redirect_project(self):
        # parse and redirect an ARK URL of a project with default project host, i.e. without specified project host
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0003")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://meta.dasch.swiss/projects/0003"

        # parse and redirect an ARK URL of a project with a specific project host
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0004")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://other-meta.dasch.swiss/projects/0004"

    def test_ark_url_info_redirect_resource(self):
        # parse and redirect an ARK URL of a DSP resource without a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0001/cmfk1DMHRBiR4-_6HXpEFA"

        # parse and redirect an ARK HTTP URL of a DSP resource without a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0001/cmfk1DMHRBiR4-_6HXpEFA"

        # parse and redirect an ARK URL of a DSP resource with a timestamp with a fractional part
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0001/cmfk1DMHRBiR4-_6HXpEFA?version=20180604T085622513Z"

        # parse and redirect an ARK URL of a DSP resource with a timestamp without a fractional part
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0001/cmfk1DMHRBiR4-_6HXpEFA?version=20180604T085622Z"

        # parse an ARK URL of a DSP resource without a timestamp and redirect it to a customized location
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0005/0_sWRg5jT3S0PLxakX9ffg1")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resources/0005/0_sWRg5jT3S0PLxakX9ffg"

    def test_ark_url_info_redirect_value(self):
        # parse an ARK URL of a DSP value without a timestamp and redirect it to a customized location
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resources/0005/SQkTPdHdTzq_gqbwj6QR-A/-SSbnPK3Q7WWxzBT1UPpRg"

        # parse and redirect an ARK URL of a DSP value with a timestamp
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0001/cmfk1DMHRBiR4-_6HXpEFA/pLlW4ODASumZfZFbJdpw1g?version=20180604T085622Z"

        # parse an ARK URL of a DSP value without a timestamp and redirect it to a customized location
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resources/0005/SQkTPdHdTzq_gqbwj6QR-A/-SSbnPK3Q7WWxzBT1UPpRg"

    def test_ark_url_info_redirect_salsah_resource(self):
        # parse and redirect a version 1 ARK URL of a PHP-SALSAH resource without a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0803/751e0b8am")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://data.dasch.swiss/resources/1"

        # parse and redirect a version 1 ARK URL of a PHP-SALSAH resource with a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0803/751e0b8am.20190118T102919Z")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://data.dasch.swiss/resources/1?citdate=20190118"

        # parse and redirect a version 0 ARK URL of a PHP-SALSAH resource without a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/080e-76bb2132d30d6-0")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://data.dasch.swiss/resources/2126045"

        # parse and redirect a version 0 ARK URL of a PHP-SALSAH resource with a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/080e-76bb2132d30d6-0.20190129")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://data.dasch.swiss/resources/2126045?citdate=20190129"

        # parse and redirect a version 0 ARK URL of a PHP-SALSAH resource with a timestamp that's too short
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/080e-76bb2132d30d6-0.2019111")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://data.dasch.swiss/resources/2126045"

    def test_ark_url_info_redirect_salsah_project(self):
        # parse and redirect an ARK URL of a project on Salsah with default project host, i.e. without specified project host
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0803")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://meta.dasch.swiss/projects/0803"

        # parse and redirect an ARK URL of a project on Salsah with a specific project host
        ark_url_info = ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0006")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://other-meta.dasch.swiss/projects/0006"

    def test_ark_url_info_redirect_salsah_ark(self):
        # parse and redirect a version 0 ARK URL of a PHP-SALSAH resource which is on DSP (migrated from salsah to DSP) without a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/0002-779b9990a0c3f-6e")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0002/Ef9heHjPWDS7dMR_gGax2Q"

        # parse and redirect a version 0 ARK URL of a PHP-SALSAH resource which is on DSP (migrated from salsah to DSP) with a timestamp
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/0002-779b9990a0c3f-6e.20190129")
        redirect_url = ark_url_info.to_redirect_url()
        assert redirect_url == "http://0.0.0.0:4200/resource/0002/Ef9heHjPWDS7dMR_gGax2Q?version=20190129"

    def test_conversion_to_resource_iri_with_ark_version_0(self):
        # convert a version 0 ARK URL to a DSP resource IRI
        ark_url_info = ArkUrlInfo(self.settings, "http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519")
        resource_iri = ark_url_info.to_resource_iri()
        assert resource_iri == "http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ"

    def test_conversion_to_resource_iri_with_ark_version_1(self):
        # convert a version 1 ARK URL to a DSP resource IRI
        ark_url_info = ArkUrlInfo(self.settings,
                                  "https://ark.example.org/ark:/00000/1/0002/0_sWRg5jT3S0PLxakX9ffg1.20210712T074927466631Z")
        resource_iri = ark_url_info.to_resource_iri()
        assert resource_iri == "http://rdfh.ch/0002/0_sWRg5jT3S0PLxakX9ffg"

    def test_reject_ark_with_wrong_digit(self):
        # reject an ARK URL that doesn't pass check digit validation
        rejected = False
        try:
            ArkUrlInfo(self.settings, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBir4=_6HXpEFAn")
        except ArkUrlException:
            rejected = True
        assert rejected
