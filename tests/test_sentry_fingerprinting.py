"""Unit tests for custom Sentry fingerprinting.

Verifies that shadow execution events and route error handlers use custom
fingerprinting to group Sentry issues by category, not by individual input.
"""

from unittest.mock import MagicMock
from unittest.mock import patch

from ark_resolver.parallel_execution import ComparisonResult
from ark_resolver.parallel_execution import ParallelExecutionResult
from ark_resolver.parallel_execution import ParallelExecutor


def _make_result(
    comparison: ComparisonResult,
    operation: str = "redirect",
    python_result: str = "http://example.com/a",
    rust_result: str = "http://example.com/b",
) -> ParallelExecutionResult:
    """Helper to create a ParallelExecutionResult for testing."""
    return ParallelExecutionResult(
        python_result=python_result,
        rust_result=rust_result,
        comparison=comparison,
        python_duration_ms=1.0,
        rust_duration_ms=0.5,
        performance_improvement_percent=50.0,
        operation=operation,
    )


class TestParallelExecutorSentryFingerprinting:
    """Tests for ParallelExecutor.track_with_sentry() fingerprinting."""

    @patch("ark_resolver.parallel_execution.sentry_sdk")
    def test_mismatch_uses_fingerprint(self, mock_sentry):
        """Verify MISMATCH events use operation-based fingerprinting."""
        mock_scope = MagicMock()
        mock_sentry.push_scope.return_value.__enter__ = MagicMock(return_value=mock_scope)
        mock_sentry.push_scope.return_value.__exit__ = MagicMock(return_value=False)

        executor = ParallelExecutor()
        result = _make_result(ComparisonResult.MISMATCH, operation="redirect")
        executor.track_with_sentry(result)

        # Verify fingerprint is set for grouping
        assert mock_scope.fingerprint == ["shadow", "redirect", "mismatch"]
        mock_scope.set_tag.assert_any_call("shadow.operation", "redirect")
        mock_scope.set_tag.assert_any_call("shadow.comparison", "mismatch")
        mock_scope.set_context.assert_called_once()
        mock_sentry.capture_message.assert_called_once()

    @patch("ark_resolver.parallel_execution.sentry_sdk")
    def test_rust_error_uses_fingerprint(self, mock_sentry):
        """Verify RUST_ERROR events use operation-based fingerprinting."""
        mock_scope = MagicMock()
        mock_sentry.push_scope.return_value.__enter__ = MagicMock(return_value=mock_scope)
        mock_sentry.push_scope.return_value.__exit__ = MagicMock(return_value=False)

        executor = ParallelExecutor()
        result = _make_result(ComparisonResult.RUST_ERROR, operation="convert")
        executor.track_with_sentry(result)

        assert mock_scope.fingerprint == ["shadow", "convert", "rust_error"]
        mock_sentry.capture_message.assert_called_once()

    @patch("ark_resolver.parallel_execution.sentry_sdk")
    def test_match_does_not_capture_message(self, mock_sentry):
        """Verify MATCH events do not send Sentry messages."""
        executor = ParallelExecutor()
        result = _make_result(ComparisonResult.MATCH)
        executor.track_with_sentry(result)

        mock_sentry.capture_message.assert_not_called()
        mock_sentry.push_scope.assert_not_called()

    @patch("ark_resolver.parallel_execution.sentry_sdk")
    def test_both_error_does_not_capture_message(self, mock_sentry):
        """Verify BOTH_ERROR events do not send Sentry messages (both failed, nothing to compare)."""
        executor = ParallelExecutor()
        result = _make_result(ComparisonResult.BOTH_ERROR)
        executor.track_with_sentry(result)

        mock_sentry.capture_message.assert_not_called()
        mock_sentry.push_scope.assert_not_called()

    @patch("ark_resolver.parallel_execution.sentry_sdk")
    def test_mismatch_truncates_results(self, mock_sentry):
        """Verify shadow_details context truncates long results."""
        mock_scope = MagicMock()
        mock_sentry.push_scope.return_value.__enter__ = MagicMock(return_value=mock_scope)
        mock_sentry.push_scope.return_value.__exit__ = MagicMock(return_value=False)

        executor = ParallelExecutor()
        result = _make_result(
            ComparisonResult.MISMATCH,
            python_result="x" * 1000,
            rust_result="y" * 1000,
        )
        executor.track_with_sentry(result)

        context_call = mock_scope.set_context.call_args
        details = context_call[0][1]
        max_len = 500
        assert len(details["python_result"]) == max_len
        assert len(details["rust_result"]) == max_len
