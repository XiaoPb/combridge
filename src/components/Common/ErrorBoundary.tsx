import { Component, ErrorInfo, ReactNode } from 'react';
import { Result, Button, Typography } from 'antd';

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
        <div style={{ padding: 24 }}>
          <Result
            status="error"
            title="页面出错了"
            subTitle="抱歉，页面遇到了一些问题。请尝试刷新页面或联系管理员。"
            extra={[
              <Button key="reset" type="primary" onClick={this.handleReset}>
                重试
              </Button>,
              <Button key="reload" onClick={() => window.location.reload()}>
                刷新页面
              </Button>,
            ]}
          >
            <div style={{ textAlign: 'left', maxWidth: 600, margin: '0 auto' }}>
              <Paragraph>
                <Text strong style={{ fontSize: 16 }}>
                  错误信息:
                </Text>
              </Paragraph>
              <Paragraph>
                <Text code>{this.state.error?.message}</Text>
              </Paragraph>
              {this.state.errorInfo && (
                <>
                  <Paragraph>
                    <Text strong style={{ fontSize: 16 }}>
                      组件堆栈:
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
                      {this.state.errorInfo.componentStack}
                    </pre>
                  </Paragraph>
                </>
              )}
            </div>
          </Result>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
