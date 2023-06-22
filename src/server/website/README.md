# Building/Running Locally

Build the image locally
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
