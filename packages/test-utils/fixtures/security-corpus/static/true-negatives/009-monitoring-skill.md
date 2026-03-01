---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean observability skill with legitimate URLs"
---
# Monitoring Setup

Configure application monitoring and alerting.

## Metrics

Expose metrics on `/metrics` in Prometheus format. Track:

- Request latency (p50, p95, p99)
- Error rate by endpoint
- Active connections
- Memory and CPU usage

## Logging

Use structured JSON logging. Include request ID, user ID,
and timestamp in every log entry. Log at appropriate levels:
DEBUG for development, INFO for production events, ERROR for failures.

## Dashboards

Create Grafana dashboards for each service. Include panels for
the four golden signals: latency, traffic, errors, and saturation.

## Alerting

Alert on error rate > 1% sustained for 5 minutes.
Alert on p99 latency > 2 seconds.
Page on-call for complete service outages.
