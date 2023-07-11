# Bazel Testing

```
ibazel run :website_bin
```

Submit the contact form:
```
curl -i -X POST -d 'subject=Test_subject_1&body=Body_1&email=' 127.0.0.1:8000/contact
```

# Building/Running the Docker Image Locally

```
# from repo root:
docker build -t website -f src/server/website/Dockerfile .
```

Run with ports exposed (-it makes it interactive, so CTRL-C works):
```
docker run -p 8000:8000 -it website
```

Debug with a shell:
```
docker run -it --entrypoint=/bin/bash website
```

Debug a failed Docker build:
```
DOCKER_BUILDKIT=0 docker build -t website --build-arg TARGETARCH="arm64" -f src/server/website/Dockerfile .
```
(TARGETARCH is only set by buildkit, but we need to disable buildkit to see
the intermediate layer ids.)

Then get the hash for a prior layer that comes after `--->` (and not after
`---> Running in ...`) and do:
```
docker run --rm -it <hash> bash
```

# Pushing

From repo root:
```
./src/server/website/copy_sources.sh ~/repos/epsilon_website
```

Then stage, commit, and push.

Follow logs:
```
doctl apps list # get id of app
doctl apps logs -f --type build <id>
```
