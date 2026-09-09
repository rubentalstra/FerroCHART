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
| `FERROCHART_TEMPLATES` | no | | A directory of `.opt` operational templates to compile at startup. |
| `FERROCHART_OVERLAYS` | no | | A directory of layouts, at most one per template. |
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

## The templates and their layouts

`FERROCHART_TEMPLATES` names a directory of `.opt` operational templates. The
server compiles each one at startup into a form and the validator that judges
what is entered against it, keyed by the identifier the template states for
itself. Compiling once is what keeps the request path cheap.

`FERROCHART_OVERLAYS` names a directory of layouts, at most one per template.
A layout is what a person decided about a form that no template states: the
order of the items, the names and the help text over the archetype's own, the
values a form starts with, and the rules that decide when an item is shown. A
deployment with no layouts draws every form in template order.

Both directories are optional. Unset means the server holds no template and
serves no form, which is the honest reading of an operator who installed none.

A directory that IS named has to hold nothing broken. A template that will not
compile fails the startup, naming the file and the cause, and so does a layout
that will not read. Two templates stating one identifier fail it too, and so
do two layouts over one template: a server that served fewer forms than its
operator installed, or picked between two by filesystem order, would be
silently wrong, and a form drawn in template order because the server dropped
a layout is the failure the overlay exists to prevent.

## The routes

No specification governs any of these. openEHR ITS-REST Release-1.1.0 defines
a CDR's API and says nothing about the API of a form server in front of one,
so every path, body and status is FerroCHART's own. What the bodies carry is
not ours to invent: they are `ferrochart-form`'s published contract, and this
surface transports them unchanged
([Write your own renderer](../evaluate/write-your-own-renderer.md)).

| Route | What it answers |
|---|---|
| `GET /health` | That this process is up, without authentication. It says nothing about the CDR or the terminology server. |
| `GET /api/templates` | The template identifiers this server holds. |
| `GET /api/templates/{template_id}/definition` | The form that template compiles to. |
| `GET /api/templates/{template_id}/layout` | The layout a person authored over it. A template nobody laid out answers with a layout that decides nothing. |
| `POST /api/templates/{template_id}/validation` | The failures in a set of entered values, keyed onto the form. It makes no request to the CDR. |
| `POST /api/ehrs/{ehr_id}/templates/{template_id}/compositions` | Builds, validates and commits. `201` with the version uid. |
| `GET /api/ehrs/{ehr_id}/compositions/{uid}/values?template={template_id}` | A stored COMPOSITION, read back into the values of that form. |

The `{uid}` takes either form ITS-REST accepts: one carrying `::` names a
version, and one without it names the versioned object and resolves to its
latest.

An unknown template is `404`, a body the route cannot read is `400`, and
values the template refuses are `422` carrying the report. A CDR that refused
or never answered is `502`, carrying the CDR's own status and body rather than
a flattened default. A composition the CDR reports as deleted is `410`.

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
{"status":"ok","version":"0.1.0"}
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
