#!/bin/bash

docker build -f domaj-server/Dockerfile -t hb.louislelievre.fr/domaj/domaj-server:latest .
docker build -t hb.louislelievre.fr/domaj/domaj-agent:latest ./domaj-agent
