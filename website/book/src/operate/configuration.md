# Configuration

<!-- toc -->

No specification governs any of this: it is FerroCHART's own design.

Every setting is an environment variable under one `FERROCHART_` namespace,
read in one place at startup. There is no configuration file and no command
line flag, because a container and an orchestrator both speak environment
variables and a second mechanism only creates a question about which one wins.

## The variables

| Variable | Required | Default | What it is |
|---|---|---|---|
| `FERROCHART_LISTEN` | no | `127.0.0.1:8080` | The address to bind. |
| `FERROCHART_CDR_URL` | yes | | The openEHR CDR's ITS-REST base URL. |
| `FERROCHART_TERM_URL` | yes | | The FHIR terminology server's base URL. |
| `FERROCHART_UI` | no | `on` | Whether the server serves the renderer at `/ui`. Reads `on` or `off` (`true`/`false`, `1`/`0`, `yes`/`no`, in any case). |
| `RUST_LOG` | no | `info` | The log filter. |

**The default bind is loopback.** A container publishes a port by widening it
to every interface in its own image, so the default never exposes a host that
did not ask for it.

**Both endpoints are required, and the server refuses to start without them.**
A form builder with no CDR to commit to and no terminology server to expand
against cannot do its work. Failing at startup with the variable's name in the
message is better than failing on the first clinician's save.

```console
$ ferrochart
ferrochart: FERROCHART_CDR_URL is not set: the openEHR CDR's ITS-REST base URL
$ echo $?
1
```

## The renderer

The published image serves the form renderer at `/ui/`, and `/ui` redirects
onto it. With the quickstart `compose.yaml` that address is
<http://127.0.0.1:8080/ui/>; change the host and port with
`FERROCHART_BIND_HOST` and `FERROCHART_PORT`.

The bundle is compiled into the `ferrochart` binary rather than copied into
the image as files, so the release archive and the container image behave the
same and a request path never reaches the filesystem. A path under `/ui` that
names a file type the bundle does not hold answers `404`; any other path
answers the single-page document, which is how a client-side route deep-links.
Content-hashed assets are served `public, max-age=31536000, immutable` and
`index.html` `no-cache`, each with its own media type and
`X-Content-Type-Options: nosniff`.

`FERROCHART_UI=off` drops the `/ui` routes for a deployment that wants an
API-only surface. A binary built without the renderer bundle serves no `/ui`
route whatever the variable says.

## The health probe

`GET /health` answers `200` with the running version, and takes no
authentication, because the thing that probes it is an orchestrator with no
credentials.

```console
$ curl -s http://127.0.0.1:8080/health
{"status":"ok","version":"0.0.2"}
```

It reports that this process is up and nothing else. It deliberately does not
check the CDR or the terminology server: a probe that fails when an upstream
is down takes a healthy process out of rotation for someone else's outage.

The published container image is distroless and carries no shell, so there is
no in-container health check to run. Probe the endpoint over HTTP from
outside, which is what an orchestrator does anyway.

## Shutdown

The process handles `SIGTERM` and `SIGINT` itself and stops serving in an
orderly way. It runs as PID 1 in the image with no shell to forward a signal,
so it has to.

## Running it

The published `compose.yaml` is the shortest path. Its default starts
FerroCHART alone against endpoints you supply, and its `demo` profile starts
FerroEHR and FerroTERM alongside it so the thing runs end to end. Those are
separately licensed images, and the demo profile is for evaluation.
