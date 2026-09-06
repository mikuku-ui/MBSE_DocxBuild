-- 追踪图基线快照（对应 DOORS Baseline）。snapshot_json 是某时刻图的冻结副本，
-- 不做查询数据源，只用于比较/回溯。
CREATE TABLE IF NOT EXISTS trace_baselines (
    id            uuid PRIMARY KEY,
    name          text NOT NULL,
    reason        text,
    snapshot_json jsonb NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now()
);
