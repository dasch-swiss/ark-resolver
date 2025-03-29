# The DSP ARK Resolver

Resolves [ARK](https://tools.ietf.org/html/draft-kunze-ark-18) URLs referring to
resources in [DSP](https://dsp.dasch.swiss/) (formerly called Knora) repositories.

## Modes of operation

The ark resolver has two modes of operation:

- When run as an HTTP server, it resolves DSP ARK URLs by redirecting
  to the actual location of each resource. Redirect URLs are generated
  from templates in a configuration file. The hostname used in the
  redirect URL, as well as the whole URL template, can be configured per
  project.
- The ark-resolver can also be used as a command-line tool for converting between
  resource IRIs and ARK URLs, using the same configuration file.

For usage information, run `just ark-cli --help`, and see the sample configuration
file `ark-config.ini` and the sample project registry file `ark-registry.ini`.

In the sample registry file, the redirect URLs are DSP-API URLs,
but it is recommended that in production, redirect URLs should refer to
human-readable representations provided by a user interface.


## Requirements / local setup

First, install `uv`, which will automatically handle your Python installations,
virtual environments, and dependencies:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Then, create the virtual environment and install the dependencies with:

```bash
uv sync
```


## Examples for using the ark-resolver on the command-line

> **Note:** The examples below are not up-to-date.

### Converting a DSP resource IRI to an ARK URL

```
$ ./ark.py -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ
https://ark.example.org/ark:/00000/1/0002/70aWaB2kWsuiN6ujYgM0ZQD
```

### Converting a DSP value IRI to an ARK URL with Timestamp

```
$ ./ark.py -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ -d 20220119T101727886178Z
https://ark.example.org/ark:/00000/1/0002/70aWaB2kWsuiN6ujYgM0ZQD.20220119T101727886178Z
```

### Converting an ARK URL from a project on salsah.org to a custom resource IRI for import into DSP

```
$ ./ark.py -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519 -r
http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ
```

### Redirecting an ARK URL from a resource created on salsah.org to the location of the resource on DSP

```
$ ./ark.py -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519
http://0.0.0.0:4200/resource/0002/70aWaB2kWsuiN6ujYgM0ZQ
```


## A note about the creation of Resource IRIs from Salsah ARK URLs
As permanent identifiers, ARKs need to be valid for an unlimited period of time. So, after resources have been migrated 
from salsah.org to DSP, their ARK URLs need to stay valid. This means that the same ARK URL that formerly was redirected 
to a resource on salsah.org, now has to be redirected to the same resource on DSP. 

To enable the correct redirection of ARK URLs coming from salsah.org to resources on DSP the DSP resource IRI 
(which contains a UUID) needs to be calculated from the resource ID provided in the ARK. To do so, UUIDs of version 5 
are used. The DaSCH specific namespace used for the creation of UUIDs is `cace8b00-717e-50d5-bcb9-486f39d733a2`. It is 
created from the generic `uuid.NAMESPACE_URL` the Python library [uuid](https://docs.python.org/3/library/uuid.html) 
provides and the string `https://dasch.swiss` and is therefore itself a UUID version 5.

Projects migrated from salsah.org to DSP need to have parameter `AllowVersion0` set to `true` in their project 
configuration (`ark-registry.ini`). Otherwise, the ARK URLs of version 0 are rejected.


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


All other GET requests are interpreted as ARK URLs.


## Using Docker

Images are published to the [daschswiss/ark-resolver](https://hub.docker.com/r/daschswiss/ark-resolver)
Docker Hub repository.

To use, run:

```bash
$ docker run daschswiss/ark-resolver
```

