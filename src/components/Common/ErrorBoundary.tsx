import { Component, ErrorInfo, ReactNode } from 'react';
import { Result, Button, Typography } from 'antd';
import { useTranslation } from 'react-i18next';

const { Paragraph, Text } = Typography;

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  onReset?: () => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

interface ErrorDisplayProps {
  error: Error | null;
  errorInfo: ErrorInfo | null;
  onReset: () => void;
}

function ErrorDisplay({ error, errorInfo, onReset }: ErrorDisplayProps) {
  const { t } = useTranslation();

  return (
    <div style={{ padding: 24 }}>
      <Result
        status="error"
        title={t('common:error.pageError')}
        subTitle={t('common:error.pageErrorDesc')}
        extra={[
          <Button key="reset" type="primary" onClick={onReset}>
            {t('common:error.retry')}
          </Button>,
          <Button key="reload" onClick={() => window.location.reload()}>
            {t('common:error.refreshPage')}
          </Button>,
        ]}
      >
        <div style={{ textAlign: 'left', maxWidth: 600, margin: '0 auto' }}>
          <Paragraph>
            <Text strong style={{ fontSize: 16 }}>
              {t('common:error.errorMessage')}:
            </Text>
          </Paragraph>
          <Paragraph>
            <Text code>{error?.message}</Text>
          </Paragraph>
          {errorInfo && (
            <>
              <Paragraph>
                <Text strong style={{ fontSize: 16 }}>
                  {t('common:error.componentStack')}:
                </Text>
              </Paragraph>
              <Paragraph>
                <pre
                  style={{
                    fontSize: 12,
                    overflow: 'auto',
                    maxHeight: 200,
                    backgroundColor: '#f5f5f5',
                    padding: 12,
                    borderRadius: 4,
                  }}
                >
                  {errorInfo.componentStack}
                </pre>
              </Paragraph>
            </>
          )}
        </div>
      </Result>
    </div>
  );
}

class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    this.setState({ errorInfo });
    console.error('ErrorBoundary caught an error:', error, errorInfo);
  }

  handleReset = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
    if (this.props.onReset) {
      this.props.onReset();
    }
  };

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <ErrorDisplay
          error={this.state.error}
          errorInfo={this.state.errorInfo}
          onReset={this.handleReset}
        />
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
