# The Knora ARK Resolver

Resolves [ARK](https://tools.ietf.org/html/draft-kunze-ark-18) URLs referring to
resources in [Knora](http://www.knora.org) repositories.

## Modes of operation

The program `ark.py` has two modes of operation:

- When run as an HTTP server, it resolves Knora ARK URLs by redirecting
  to the actual location of each resource. Redirect URLs are generated
  from templates in a configuration file. The hostname used in the
  redirect URL, as well as the whole URL template, can be configured per
  project.

- It can also be used as a command-line tool for converting between
  resource IRIs and ARK URLs, using the same configuration file.

For usage information, run `./ark.py --help`, and see the sample configuration
file `ark-config.ini` and the sample project registry file `ark-registry.ini`.

In the sample registry file, the redirect URLs are Knora API URLs,
but it is recommended that in production, redirect URLs should refer to
human-readable representations provided by a user interface.

Prerequisites:

- Python 3
- [Sanic](https://sanic.readthedocs.io/en/latest/)
- [Requests](http://docs.python-requests.org/en/master/)

## Server routes

```
GET /config
```

Returns the server's configuration, including the project registry, but not
including `ArkGitHubSecret`.

```
POST /reload
```

Accepts a GitHub webhook request in JSON, and validates it according to
[Securing your webhooks](https://developer.github.com/webhooks/securing/), using
the secret configured as `ArkGitHubSecret`. If the request is valid, reloads the
configuration, including the project registry. Changes to `ArkInternalHost` and
`ArkInternalPort` are not taken into account.

```
GET /make_php_ark_url?project_id=PROJECT_ID&resource_id=RESOURCE_ID
```

Takes a project ID (a hexadecimal number) and a PHP-SALSAH resource ID (an integer in base 10)
and returns an ARK URL.

All other GET requests are interpreted as ARK URLs.

## Using Docker

Images are published to the [dhlab-basel/ark-resolver](https://cloud.docker.com/u/dhlabbasel/repository/docker/dhlabbasel/ark-resolver)
Docker Hub repository.

To use, run:

```bash
$ docker run daschswiss/ark-resolver
```

## Requirements

To install the requirements:

```bash
$ pip3 install -r requirements.txt
```


To generate a "requirements" file (usually requirements.txt), that you commit with your project, do:

```bash
$ pip3 freeze > requirements.txt
```
