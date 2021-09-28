# The DSP ARK Resolver

Resolves [ARK](https://tools.ietf.org/html/draft-kunze-ark-18) URLs referring to
resources in [DSP](https://dsp.dasch.swiss/) (formerly called Knora) repositories.

## Modes of operation

The program `ark.py` has two modes of operation:

- When run as an HTTP server, it resolves DSP ARK URLs by redirecting
  to the actual location of each resource. Redirect URLs are generated
  from templates in a configuration file. The hostname used in the
  redirect URL, as well as the whole URL template, can be configured per
  project.

- It can also be used as a command-line tool for converting between
  resource IRIs and ARK URLs, using the same configuration file.

For usage information, run `./ark.py --help`, and see the sample configuration
file `ark-config.ini` and the sample project registry file `ark-registry.ini`.

In the sample registry file, the redirect URLs are DSP API URLs,
but it is recommended that in production, redirect URLs should refer to
human-readable representations provided by a user interface.

Prerequisites:

- Python 3
- [Sanic](https://sanic.readthedocs.io/en/latest/)
- [Requests](http://docs.python-requests.org/en/master/)

## Converting an ARK URL from a project on salsah.org to a custom resource IRI for import into DSP

```
$ ./ark.py -r -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519
http://rdfh.ch/0002/751e0b8a
$ ./ark.py -r -p 080E -n 1                                                
http://rdfh.ch/080E/751e0b8a
```

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

Images are published to the [daschswiss/ark-resolver](https://hub.docker.com/r/daschswiss/ark-resolver)
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

To generate the requirements file (requirements.txt), that you commit with the project, do:

```bash
$ pip3 freeze > requirements.txt
```
