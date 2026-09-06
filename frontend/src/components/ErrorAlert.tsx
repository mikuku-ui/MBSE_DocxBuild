import { Alert } from 'antd';

interface ErrorAlertProps {
  error: string | null | undefined;
  /** 额外类（如外边距 mt-2），透传给 Alert 根元素 */
  className?: string;
}

/** 统一的加载错误条：无错误返回 null，有错误渲染一个 error Alert。 */
export default function ErrorAlert({ error, className }: ErrorAlertProps) {
  if (!error) return null;
  return <Alert type="error" showIcon message={error} className={className} />;
}
