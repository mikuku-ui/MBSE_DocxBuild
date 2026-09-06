// 跨页共用的小工具：日期格式化 + 从 RTK/接口错误里提取可读 message。

/** 时间字段 → 本地可读串：空回退「-」，非法时间回退原文（各页原 fmt 的合并语义）。 */
export function formatDateTime(v: string | null | undefined): string {
  if (!v) return '-';
  const d = new Date(v);
  return Number.isNaN(d.getTime()) ? v : d.toLocaleString();
}

/** 从 catch 到的值里取出可读错误信息。RTK `.unwrap()` 的拒绝值常是序列化后的
 *  普通对象 `{ name, message }`，`String(e)` 会变成 `[object Object]`，这里统一取出 message。 */
export function errMsg(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (e && typeof e === 'object' && 'message' in e) {
    const m = (e as { message?: unknown }).message;
    if (typeof m === 'string' && m) return m;
  }
  return '操作失败';
}
