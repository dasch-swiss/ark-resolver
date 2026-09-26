"""HEAD on an ARK answers exactly as GET does: the same status and the same Location."""

import os
from http import HTTPStatus

import pytest
from ark_resolver._rust import load_settings as load_settings_rust  # type: ignore[import-untyped]
from sanic import Sanic

from ark_resolver import ark
from ark_resolver.routes.redirect import redirect_bp


@pytest.fixture(scope="module")
def app() -> Sanic:
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    test_app = Sanic("ark_resolver_head_test")
    test_app.blueprint(redirect_bp)
    test_app.config.settings = ark.load_settings()
    test_app.config.rust_settings = load_settings_rust()
    return test_app


@pytest.mark.parametrize(
    "ark_id",
    [
        "ark:/00000/1",
        "ark:/00000/1/0003",
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn",
        "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z",
    ],
)
def test_head_redirects_like_get(app: Sanic, ark_id: str) -> None:
    _, get = app.test_client.get(f"/{ark_id}", allow_redirects=False)
    _, head = app.test_client.head(f"/{ark_id}", allow_redirects=False)

    assert HTTPStatus(get.status).is_redirection
    assert head.status == get.status
    assert head.headers["location"] == get.headers["location"]


def test_head_on_an_invalid_ark_answers_like_get(app: Sanic) -> None:
    _, get = app.test_client.get("/ark:/00000/1/0001/invalid", allow_redirects=False)
    _, head = app.test_client.head("/ark:/00000/1/0001/invalid", allow_redirects=False)

    assert HTTPStatus(get.status).is_client_error
    assert head.status == get.status
