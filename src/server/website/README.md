# Bazel Testing

```
ibazel run :website_bin
```

Submit the contact form. Locally the server uses Cloudflare's Turnstile test
keys, so any token is accepted, but the field must be present:
```
curl -i -X POST -d 'subject=Test_subject_1&body=Body_1&email=&cf-turnstile-response=x' 127.0.0.1:8000/contact
```

In-app problem reports (see `aft/routes/contact.rs`) skip the Turnstile check.
In production the keys come from `APP_TURNSTILE__SITE_KEY` and
`APP_TURNSTILE__SECRET_KEY` (see `spec.yaml`); the widget is created in the
Cloudflare dashboard under Turnstile for `scoria.info`.

# Cross Compiling

The container image is always built for linux/amd64, regardless of the
platform flags on the command line:

```
bazel build -c opt :image_amd64
```

The `linux_amd64_files` rule in `//src/bzl:platforms.bzl` applies the
transition. It is equivalent to passing these flags:
```
--platforms @zig_sdk//platform:linux_amd64
--extra_toolchains @zig_sdk//toolchain:linux_amd64_gnu.2.34
```

## Running the amd64 Container Locally

```
bazel run -c opt :load_amd64
docker run -p 8000:8000 -it epsln/site:latest
```

(-it makes it interactive, so CTRL-C works)

Debug with a shell:
```
docker run -it --entrypoint=/bin/bash website
```

### Colima Docker Runtime

```
colima start
```

If you get a platform mismatch error, try `colima stop`, `rm -rf ~/.colima`,
and then `colima start`.

## Pushing the Container

```
doctl auth init && doctl registry login
bazel run -c opt :push_amd64
```

Follow logs:
```
doctl apps list # get id of app
doctl apps logs -f --type build <id>
```

## Updating the App

Place the new version in `static/apk/`, then add the files to the
`files_and_envs` declaration in the BUILD file. Then add a new route to serve
the file in `aft/routes/apk_download.rs`. Update the `latest` redirect to the
new version.
