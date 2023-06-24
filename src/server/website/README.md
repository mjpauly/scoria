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
