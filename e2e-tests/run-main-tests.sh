#!/usr/bin/env bash

if [ "$ENABLE_SETUP" = "true" ]; then
  node setup.js
fi
k6 run -e "API_ORIGIN=$API_ORIGIN" -e "TARGET_URL=$TARGET_URL" -e "SERVICE_TOKEN=$MASTER_API_KEY" -e "ORGANIZATION_ID=$ORGANIZATION_ID" -e "VUS=$VUS" -e "ITERATIONS=$ITERATIONS" -e "DURATION=$DURATION" main.js
