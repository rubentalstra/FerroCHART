# syntax=docker/dockerfile:1
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1

# The FerroCHART image: one static binary on distroless static, per platform.
#
# The binary is not built here. The release workflow builds and attests it in
# the SLSA Build L3 lane (.github/workflows/release-build.yml), verifies that
# attestation, and stages the musl binary under dist/<os>/<arch>/ before this
# file is built for linux/amd64 and linux/arm64
# (.github/workflows/release-image.yml). There is no build stage, so nothing
# executes while the image is assembled and the arm64 manifest needs no QEMU.
#
# Build locally from a staged dist/ tree, from the repository root:
#   docker buildx build -f docker/Dockerfile --platform linux/arm64 .

# distroless static: ca-certificates, tzdata, an /etc/passwd carrying the
# nonroot user (uid 65532), a /tmp, no shell, and no package manager. Pinned by
# index digest; Dependabot (docker ecosystem) proposes the bumps.
FROM gcr.io/distroless/static-debian13:nonroot@sha256:1c2c046bc09ed40fad370b599a0b1ae7987f55b01e247cf27a7c27cd97e5bbc7

# The platform of the manifest being built, set by buildx per platform.
ARG TARGETOS
ARG TARGETARCH

# Root-owned and not writable by the runtime user, so the process cannot
# rewrite its own binary.
COPY --chmod=0555 dist/${TARGETOS}/${TARGETARCH}/ferrochart /usr/local/bin/ferrochart

# A container cannot publish a loopback bind, so the image default is every
# interface on the documented port.
ENV FERROCHART_LISTEN=0.0.0.0:8080
EXPOSE 8080/tcp

# Numeric on purpose: the kubelet cannot resolve a user name against an image
# it does not read, so `runAsNonRoot: true` refuses a named USER.
USER 65532:65532

# Exec form: the binary is PID 1 and receives SIGTERM itself, because there is
# no shell in the image to forward it.
ENTRYPOINT ["/usr/local/bin/ferrochart"]

# Build-independent OCI labels; the release workflow adds the build-dependent
# ones (version, revision, created) and mirrors every label as an index
# annotation, which is what GHCR reads for the package page.
LABEL org.opencontainers.image.title="FerroCHART" \
      org.opencontainers.image.description="A pure-Rust openEHR form builder and renderer" \
      org.opencontainers.image.url="https://ferrochart.eu" \
      org.opencontainers.image.documentation="https://ferrochart.eu/docs/" \
      org.opencontainers.image.source="https://github.com/rubentalstra/FerroCHART" \
      org.opencontainers.image.vendor="FerroCHART" \
      org.opencontainers.image.licenses="BUSL-1.1" \
      org.opencontainers.image.base.name="gcr.io/distroless/static-debian13:nonroot"
