import os

from opentelemetry import trace
from opentelemetry.propagate import set_global_textmap
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import ConsoleSpanExporter
from opentelemetry.sdk.trace.export import SimpleSpanProcessor
from sentry_sdk.integrations.opentelemetry import SentryPropagator
from sentry_sdk.integrations.opentelemetry import SentrySpanProcessor

#################################################################################################
# OpenTelemetry

provider = TracerProvider()

# Add both Sentry and Console span processors to the provider
provider.add_span_processor(SentrySpanProcessor())  # Sentry integration

# Add console exporter for local debugging only if explicitly enabled
if os.environ.get("ARK_TRACING_CONSOLE", "FALSE").lower() == "true":
    console_exporter = ConsoleSpanExporter()
    provider.add_span_processor(SimpleSpanProcessor(console_exporter))

set_global_textmap(SentryPropagator())

# Sets the global default tracer provider
trace.set_tracer_provider(provider)

# Creates a tracer from the global tracer provider
tracer = trace.get_tracer("ark-resolver")
