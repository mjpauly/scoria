# Bazel Testing

```
ibazel run :website_bin
```

Submit the contact form:
```
curl -i -X POST -d 'subject=Test_subject_1&body=Body_1&email=' 127.0.0.1:8000/contact
```

# Cross Compiling

```
bazel build --config=linux_amd64 :website_bin
```

The config is defined in the `.bazelrc`. It just sets these flags for each bazel
invocation:
```
--platforms @zig_sdk//platform:linux_amd64
--extra_toolchains @zig_sdk//toolchain:linux_amd64_gnu.2.34
```

## Running the amd64 Container Locally

```
bazel run --config=linux_amd64 -c opt :image_amd64
docker run -p 8000:8000 -it bazel/src/server/website:image_amd64
```

(-it makes it interactive, so CTRL-C works)

Debug with a shell:
```
docker run -it --entrypoint=/bin/bash website
```

## Pushing the Container

```
doctl auth init && doctl registry login
bazel run --config=linux_amd64 -c opt :push_amd64
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
