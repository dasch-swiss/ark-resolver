#!/usr/bin/env python3

# Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
# SPDX-License-Identifier: Apache-2.0

"""
Parallel execution framework for running Python and Rust implementations in production.

This module provides infrastructure for executing both Python and Rust implementations
of ARK URL processing functions in parallel, using Python as the primary implementation
and Rust as a shadow for validation and performance measurement.
"""

import logging
import time
from dataclasses import dataclass
from enum import Enum
from typing import Any
from typing import Callable
from typing import Dict
from typing import Optional
from typing import Tuple

import sentry_sdk
from opentelemetry.trace import Span
from sanic.log import logger


class ComparisonResult(Enum):
    """Result of comparing Python and Rust execution."""

    MATCH = "match"
    MISMATCH = "mismatch"
    RUST_ERROR = "rust_error"
    PYTHON_ERROR = "python_error"
    BOTH_ERROR = "both_error"


@dataclass
class ParallelExecutionResult:
    """Result of parallel execution with performance metrics."""

    python_result: Any
    rust_result: Any
    comparison: ComparisonResult
    python_duration_ms: float
    rust_duration_ms: float
    performance_improvement_percent: float
    operation: str
    error_details: str = ""
    python_error: Optional[Exception] = None
    rust_error: Optional[Exception] = None


class ParallelExecutor:
    """
    Executes Python and Rust implementations in parallel for validation and performance measurement.

    This class provides a framework for running both implementations, comparing results,
    and collecting performance metrics while ensuring that Rust errors don't affect
    the user experience (Python remains primary).
    """

    def __init__(self, logger: logging.Logger = logger):
        self.logger = logger
        self.metrics: Dict[str, int] = {
            "total_executions": 0,
            "matches": 0,
            "mismatches": 0,
            "rust_errors": 0,
            "python_errors": 0,
            "both_errors": 0,
        }

    def execute_parallel(
        self, operation: str, python_func: Callable, rust_func: Callable, *args, **kwargs
    ) -> Tuple[Any, ParallelExecutionResult]:
        """
        Execute both Python and Rust implementations in parallel.

        Args:
            operation: Name of the operation being executed
            python_func: Python implementation function
            rust_func: Rust implementation function
            *args: Arguments to pass to both functions
            **kwargs: Keyword arguments to pass to both functions

        Returns:
            Tuple of (python_result, execution_result)
            The python_result is always returned for user-facing responses
        """
        self.metrics["total_executions"] += 1

        # Execute Python implementation (primary)
        python_start = time.perf_counter()
        python_result = None
        python_error = None

        try:
            python_result = python_func(*args, **kwargs)
        except Exception as e:
            python_error = e
            self.logger.error(f"Python execution failed for {operation}: {e}", exc_info=True)

        python_duration = time.perf_counter() - python_start

        # Execute Rust implementation (shadow)
        rust_start = time.perf_counter()
        rust_result = None
        rust_error = None

        try:
            rust_result = rust_func(*args, **kwargs)
        except Exception as e:  # noqa: BLE001
            rust_error = e
            self.logger.warning(f"Rust execution failed for {operation}: {e}", exc_info=True)

        rust_duration = time.perf_counter() - rust_start

        # Determine comparison result
        comparison = self._compare_results(python_result, rust_result, python_error, rust_error)

        # Calculate performance improvement
        performance_improvement = 0.0
        if python_duration > 0 and rust_duration > 0 and not python_error and not rust_error:
            performance_improvement = ((python_duration - rust_duration) / python_duration) * 100

        # Create execution result
        execution_result = ParallelExecutionResult(
            python_result=python_result,
            rust_result=rust_result,
            comparison=comparison,
            python_duration_ms=python_duration * 1000,
            rust_duration_ms=rust_duration * 1000,
            performance_improvement_percent=performance_improvement,
            operation=operation,
            error_details=self._format_error_details(python_error, rust_error),
            python_error=python_error,
            rust_error=rust_error,
        )

        # Log and track metrics
        self._log_execution_result(execution_result)
        self._track_metrics(comparison)

        # If Python failed, re-raise the exception
        if python_error:
            raise python_error

        return python_result, execution_result

    def add_to_span(self, span: Span, execution_result: ParallelExecutionResult) -> None:
        """
        Add parallel execution metrics to an OpenTelemetry span.

        Args:
            span: OpenTelemetry span to add attributes to
            execution_result: Result of parallel execution
        """
        span.set_attribute("parallel.operation", execution_result.operation)
        span.set_attribute("parallel.python_duration_ms", execution_result.python_duration_ms)
        span.set_attribute("parallel.rust_duration_ms", execution_result.rust_duration_ms)
        span.set_attribute("parallel.performance_improvement_percent", execution_result.performance_improvement_percent)
        span.set_attribute("parallel.comparison_result", execution_result.comparison.value)
        span.set_attribute("parallel.results_match", execution_result.comparison == ComparisonResult.MATCH)

        if execution_result.error_details:
            span.set_attribute("parallel.error_details", execution_result.error_details)

    def track_with_sentry(self, execution_result: ParallelExecutionResult) -> None:
        """
        Track parallel execution metrics with Sentry.

        Args:
            execution_result: Result of parallel execution
        """
        sentry_sdk.set_tag("parallel.operation", execution_result.operation)
        sentry_sdk.set_tag("parallel.comparison_result", execution_result.comparison.value)
        sentry_sdk.set_tag("parallel.results_match", execution_result.comparison == ComparisonResult.MATCH)

        sentry_sdk.set_measurement("parallel.python_duration_ms", execution_result.python_duration_ms)
        sentry_sdk.set_measurement("parallel.rust_duration_ms", execution_result.rust_duration_ms)
        sentry_sdk.set_measurement("parallel.performance_improvement_percent", execution_result.performance_improvement_percent)

        # Track mismatches as custom events
        if execution_result.comparison == ComparisonResult.MISMATCH:
            sentry_sdk.capture_message(
                f"Parallel execution mismatch in {execution_result.operation}",
                level="warning",
                extras={
                    "python_result": str(execution_result.python_result),
                    "rust_result": str(execution_result.rust_result),
                    "performance_improvement": execution_result.performance_improvement_percent,
                },
            )

    def get_metrics_summary(self) -> Dict[str, Any]:
        """
        Get a summary of parallel execution metrics.

        Returns:
            Dictionary containing execution metrics
        """
        total = self.metrics["total_executions"]
        if total == 0:
            return {"total_executions": 0}

        return {
            "total_executions": total,
            "match_rate_percent": (self.metrics["matches"] / total) * 100,
            "mismatch_rate_percent": (self.metrics["mismatches"] / total) * 100,
            "rust_error_rate_percent": (self.metrics["rust_errors"] / total) * 100,
            "python_error_rate_percent": (self.metrics["python_errors"] / total) * 100,
            "both_error_rate_percent": (self.metrics["both_errors"] / total) * 100,
            "detailed_counts": self.metrics.copy(),
        }

    def _compare_results(
        self, python_result: Any, rust_result: Any, python_error: Optional[Exception], rust_error: Optional[Exception]
    ) -> ComparisonResult:
        """Compare results from Python and Rust implementations."""
        if python_error and rust_error:
            return ComparisonResult.BOTH_ERROR
        elif python_error:
            return ComparisonResult.PYTHON_ERROR
        elif rust_error:
            return ComparisonResult.RUST_ERROR
        elif python_result == rust_result:
            return ComparisonResult.MATCH
        else:
            return ComparisonResult.MISMATCH

    def _format_error_details(self, python_error: Optional[Exception], rust_error: Optional[Exception]) -> str:
        """Format error details for logging."""
        details = []
        if python_error:
            details.append(f"Python: {type(python_error).__name__}: {python_error}")
        if rust_error:
            details.append(f"Rust: {type(rust_error).__name__}: {rust_error}")
        return "; ".join(details)

    def _log_execution_result(self, execution_result: ParallelExecutionResult) -> None:
        """Log the result of parallel execution."""
        log_data = {
            "operation": execution_result.operation,
            "python_duration_ms": execution_result.python_duration_ms,
            "rust_duration_ms": execution_result.rust_duration_ms,
            "performance_improvement_percent": execution_result.performance_improvement_percent,
            "comparison_result": execution_result.comparison.value,
            "results_match": execution_result.comparison == ComparisonResult.MATCH,
        }

        if execution_result.comparison == ComparisonResult.MATCH:
            if execution_result.performance_improvement_percent > 0:
                self.logger.info(
                    f"Parallel execution SUCCESS: {execution_result.operation} - "
                    f"Rust {execution_result.performance_improvement_percent:.1f}% faster",
                    extra=log_data,
                )
            else:
                self.logger.info(f"Parallel execution SUCCESS: {execution_result.operation} - Results match", extra=log_data)
        elif execution_result.comparison == ComparisonResult.MISMATCH:
            self.logger.warning(
                f"Parallel execution MISMATCH: {execution_result.operation} - "
                f"Python: {execution_result.python_result}, Rust: {execution_result.rust_result}",
                extra=log_data,
            )
        else:
            self.logger.warning(
                f"Parallel execution ERROR: {execution_result.operation} - {execution_result.error_details}", extra=log_data
            )

    def _track_metrics(self, comparison: ComparisonResult) -> None:
        """Track execution metrics."""
        if comparison == ComparisonResult.MATCH:
            self.metrics["matches"] += 1
        elif comparison == ComparisonResult.MISMATCH:
            self.metrics["mismatches"] += 1
        elif comparison == ComparisonResult.RUST_ERROR:
            self.metrics["rust_errors"] += 1
        elif comparison == ComparisonResult.PYTHON_ERROR:
            self.metrics["python_errors"] += 1
        elif comparison == ComparisonResult.BOTH_ERROR:
            self.metrics["both_errors"] += 1


# Global instance for use throughout the application
parallel_executor = ParallelExecutor()
