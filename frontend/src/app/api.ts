// 极简 fetch JSON 客户端。后端错误统一为 { error, message }，这里抛出 message。

const BASE = '/api';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(BASE + path, {
      headers: { 'Content-Type': 'application/json' },
      ...init,
    });
  } catch {
    throw new Error('无法连接后端，请确认服务已启动（cargo run）');
  }
  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = (await res.json()) as { error?: string; message?: string };
      if (body.message) message = body.message;
    } catch {
      /* 非 JSON 响应，保留状态文本 */
    }
    throw new Error(message);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

function bodyOf(value: unknown): string | undefined {
  return value === undefined ? undefined : JSON.stringify(value);
}

export const api = {
  get: <T,>(path: string): Promise<T> => request<T>(path),
  post: <T,>(path: string, value?: unknown): Promise<T> =>
    request<T>(path, { method: 'POST', body: bodyOf(value) }),
  put: <T,>(path: string, value?: unknown): Promise<T> =>
    request<T>(path, { method: 'PUT', body: bodyOf(value) }),
  del: <T,>(path: string): Promise<T> => request<T>(path, { method: 'DELETE' }),
};
