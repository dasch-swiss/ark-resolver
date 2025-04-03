from opentelemetry import trace
from opentelemetry.propagate import set_global_textmap
from opentelemetry.sdk.trace import TracerProvider
from sentry_sdk.integrations.opentelemetry import SentryPropagator
from sentry_sdk.integrations.opentelemetry import SentrySpanProcessor

#################################################################################################
# OpenTelemetry

provider = TracerProvider()

# Add both Sentry and Console span processors to the provider
provider.add_span_processor(SentrySpanProcessor())  # Sentry integration

set_global_textmap(SentryPropagator())

# Sets the global default tracer provider
trace.set_tracer_provider(provider)

# Creates a tracer from the global tracer provider
tracer = trace.get_tracer("ark-resolver")
