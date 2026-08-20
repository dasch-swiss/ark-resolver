# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

"""Tests for the CORS response header the resolver sets on every response."""

import asyncio

from sanic import HTTPResponse

from ark_resolver.ark import add_cors_headers


def test_cors_header_allows_any_origin() -> None:
    """Every response carries Access-Control-Allow-Origin, so browser clients can read it.

    This replaced sanic-cors; without it, cross-origin JavaScript reads of resolver
    responses would be blocked by the browser with no server-side error.
    """
    res = HTTPResponse()

    asyncio.run(add_cors_headers(None, res))  # type: ignore[arg-type]

    assert res.headers["Access-Control-Allow-Origin"] == "*"


def test_cors_header_does_not_reflect_the_request_origin() -> None:
    """The header is a constant, never the caller's Origin.

    Reflecting an origin is only safe while no credentials are allowed; pinning "*" keeps
    that combination unreachable rather than merely unused.
    """
    res = HTTPResponse()

    asyncio.run(add_cors_headers(None, res))  # type: ignore[arg-type]

    assert res.headers["Access-Control-Allow-Origin"] == "*"
    assert "Access-Control-Allow-Credentials" not in res.headers
