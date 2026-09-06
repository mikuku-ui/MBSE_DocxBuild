--
-- PostgreSQL database dump
--

\restrict gTchnenc41teAuiXDveeVaZN2xJG11pUf7x4KnTyyu6hJJ66x1cRhVQOGBtog3l

-- Dumped from database version 15.16 (Debian 15.16-0+deb12u1)
-- Dumped by pg_dump version 16.15

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: mbsetest; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA mbsetest;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: document_template_edges; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.document_template_edges (
    id uuid NOT NULL,
    template_id uuid NOT NULL,
    source_node_id uuid NOT NULL,
    target_node_id uuid NOT NULL,
    edge_type text DEFAULT 'contains'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT document_template_edges_check CHECK ((source_node_id <> target_node_id))
);


--
-- Name: document_template_nodes; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.document_template_nodes (
    id uuid NOT NULL,
    template_id uuid NOT NULL,
    node_type text NOT NULL,
    title text NOT NULL,
    content_spec jsonb DEFAULT '{}'::jsonb NOT NULL,
    tex_component text,
    sort_key integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: document_templates; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.document_templates (
    id uuid NOT NULL,
    name text NOT NULL,
    kind text NOT NULL,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: execution_instances; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.execution_instances (
    id uuid NOT NULL,
    template_id uuid NOT NULL,
    stage text,
    status text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    project_id uuid NOT NULL
);


--
-- Name: execution_items; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.execution_items (
    id uuid NOT NULL,
    instance_id uuid NOT NULL,
    template_node_id uuid NOT NULL,
    title text NOT NULL,
    status text NOT NULL,
    content jsonb DEFAULT '{}'::jsonb NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_basis_files; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_basis_files (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    category text NOT NULL,
    seq integer DEFAULT 0 NOT NULL,
    name text NOT NULL,
    ident text DEFAULT ''::text NOT NULL,
    version text DEFAULT ''::text NOT NULL,
    pub_date text DEFAULT ''::text NOT NULL,
    pub_org text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_devices; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_devices (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    seq integer DEFAULT 0 NOT NULL,
    name text NOT NULL,
    device_type text DEFAULT ''::text NOT NULL,
    safety_level text DEFAULT ''::text NOT NULL,
    run_env text DEFAULT ''::text NOT NULL,
    dev_env text DEFAULT ''::text NOT NULL,
    language text DEFAULT ''::text NOT NULL,
    version text DEFAULT ''::text NOT NULL,
    code_scale text DEFAULT ''::text NOT NULL,
    dev_org text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_environment_items; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_environment_items (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    category text NOT NULL,
    resource_kind text DEFAULT ''::text NOT NULL,
    name text NOT NULL,
    version_config text DEFAULT ''::text NOT NULL,
    qty text DEFAULT ''::text NOT NULL,
    note text DEFAULT ''::text NOT NULL,
    provider text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_field_values; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_field_values (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    field_id uuid NOT NULL,
    value text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_ledger_fields; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_ledger_fields (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    doc_kind text DEFAULT 'project'::text NOT NULL,
    field_key text NOT NULL,
    value text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_parties; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_parties (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    party_kind text NOT NULL,
    name text NOT NULL,
    address text DEFAULT ''::text NOT NULL,
    contact text DEFAULT ''::text NOT NULL,
    phone text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_table_rows; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_table_rows (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    table_id uuid NOT NULL,
    row_index integer DEFAULT 0 NOT NULL,
    cells jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: project_test_schedule; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.project_test_schedule (
    id uuid NOT NULL,
    project_id uuid NOT NULL,
    seq integer DEFAULT 0 NOT NULL,
    phase text NOT NULL,
    time_range text DEFAULT ''::text NOT NULL,
    location text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: projects; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.projects (
    id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    scheme_id uuid
);


--
-- Name: scheme_fields; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.scheme_fields (
    id uuid NOT NULL,
    scheme_id uuid NOT NULL,
    doc_kind text DEFAULT 'project'::text NOT NULL,
    field_key text NOT NULL,
    label text DEFAULT ''::text NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: scheme_table_columns; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.scheme_table_columns (
    id uuid NOT NULL,
    table_id uuid NOT NULL,
    column_key text NOT NULL,
    label text DEFAULT ''::text NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: scheme_tables; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.scheme_tables (
    id uuid NOT NULL,
    scheme_id uuid NOT NULL,
    table_key text NOT NULL,
    label text DEFAULT ''::text NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: schemes; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.schemes (
    id uuid NOT NULL,
    name text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: trace_baselines; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.trace_baselines (
    id uuid NOT NULL,
    name text NOT NULL,
    reason text,
    snapshot_json jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    project_id uuid NOT NULL
);


--
-- Name: trace_links; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.trace_links (
    id uuid NOT NULL,
    source_node_id uuid NOT NULL,
    target_node_id uuid NOT NULL,
    link_type text NOT NULL,
    attributes jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    project_id uuid NOT NULL,
    CONSTRAINT trace_links_check CHECK ((source_node_id <> target_node_id))
);


--
-- Name: trace_nodes; Type: TABLE; Schema: mbsetest; Owner: -
--

CREATE TABLE mbsetest.trace_nodes (
    id uuid NOT NULL,
    kind text NOT NULL,
    external_source text NOT NULL,
    external_ref text NOT NULL,
    module text,
    title text NOT NULL,
    description text,
    attributes jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    sort_key integer DEFAULT 0 NOT NULL,
    project_id uuid NOT NULL
);


--
-- Name: document_template_edges document_template_edges_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_edges
    ADD CONSTRAINT document_template_edges_pkey PRIMARY KEY (id);


--
-- Name: document_template_edges document_template_edges_template_id_source_node_id_target_n_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_edges
    ADD CONSTRAINT document_template_edges_template_id_source_node_id_target_n_key UNIQUE (template_id, source_node_id, target_node_id, edge_type);


--
-- Name: document_template_nodes document_template_nodes_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_nodes
    ADD CONSTRAINT document_template_nodes_pkey PRIMARY KEY (id);


--
-- Name: document_templates document_templates_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_templates
    ADD CONSTRAINT document_templates_pkey PRIMARY KEY (id);


--
-- Name: execution_instances execution_instances_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.execution_instances
    ADD CONSTRAINT execution_instances_pkey PRIMARY KEY (id);


--
-- Name: execution_items execution_items_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.execution_items
    ADD CONSTRAINT execution_items_pkey PRIMARY KEY (id);


--
-- Name: project_basis_files project_basis_files_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_basis_files
    ADD CONSTRAINT project_basis_files_pkey PRIMARY KEY (id);


--
-- Name: project_devices project_devices_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_devices
    ADD CONSTRAINT project_devices_pkey PRIMARY KEY (id);


--
-- Name: project_environment_items project_environment_items_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_environment_items
    ADD CONSTRAINT project_environment_items_pkey PRIMARY KEY (id);


--
-- Name: project_field_values project_field_values_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_field_values
    ADD CONSTRAINT project_field_values_pkey PRIMARY KEY (id);


--
-- Name: project_field_values project_field_values_project_id_field_id_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_field_values
    ADD CONSTRAINT project_field_values_project_id_field_id_key UNIQUE (project_id, field_id);


--
-- Name: project_ledger_fields project_ledger_fields_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_ledger_fields
    ADD CONSTRAINT project_ledger_fields_pkey PRIMARY KEY (id);


--
-- Name: project_ledger_fields project_ledger_fields_project_id_doc_kind_field_key_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_ledger_fields
    ADD CONSTRAINT project_ledger_fields_project_id_doc_kind_field_key_key UNIQUE (project_id, doc_kind, field_key);


--
-- Name: project_parties project_parties_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_parties
    ADD CONSTRAINT project_parties_pkey PRIMARY KEY (id);


--
-- Name: project_parties project_parties_project_id_party_kind_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_parties
    ADD CONSTRAINT project_parties_project_id_party_kind_key UNIQUE (project_id, party_kind);


--
-- Name: project_table_rows project_table_rows_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_table_rows
    ADD CONSTRAINT project_table_rows_pkey PRIMARY KEY (id);


--
-- Name: project_table_rows project_table_rows_project_id_table_id_row_index_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_table_rows
    ADD CONSTRAINT project_table_rows_project_id_table_id_row_index_key UNIQUE (project_id, table_id, row_index);


--
-- Name: project_test_schedule project_test_schedule_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_test_schedule
    ADD CONSTRAINT project_test_schedule_pkey PRIMARY KEY (id);


--
-- Name: projects projects_name_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.projects
    ADD CONSTRAINT projects_name_key UNIQUE (name);


--
-- Name: projects projects_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.projects
    ADD CONSTRAINT projects_pkey PRIMARY KEY (id);


--
-- Name: scheme_fields scheme_fields_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_fields
    ADD CONSTRAINT scheme_fields_pkey PRIMARY KEY (id);


--
-- Name: scheme_fields scheme_fields_scheme_id_doc_kind_field_key_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_fields
    ADD CONSTRAINT scheme_fields_scheme_id_doc_kind_field_key_key UNIQUE (scheme_id, doc_kind, field_key);


--
-- Name: scheme_table_columns scheme_table_columns_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_table_columns
    ADD CONSTRAINT scheme_table_columns_pkey PRIMARY KEY (id);


--
-- Name: scheme_table_columns scheme_table_columns_table_id_column_key_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_table_columns
    ADD CONSTRAINT scheme_table_columns_table_id_column_key_key UNIQUE (table_id, column_key);


--
-- Name: scheme_tables scheme_tables_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_tables
    ADD CONSTRAINT scheme_tables_pkey PRIMARY KEY (id);


--
-- Name: scheme_tables scheme_tables_scheme_id_table_key_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_tables
    ADD CONSTRAINT scheme_tables_scheme_id_table_key_key UNIQUE (scheme_id, table_key);


--
-- Name: schemes schemes_name_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.schemes
    ADD CONSTRAINT schemes_name_key UNIQUE (name);


--
-- Name: schemes schemes_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.schemes
    ADD CONSTRAINT schemes_pkey PRIMARY KEY (id);


--
-- Name: trace_baselines trace_baselines_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_baselines
    ADD CONSTRAINT trace_baselines_pkey PRIMARY KEY (id);


--
-- Name: trace_links trace_links_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_links
    ADD CONSTRAINT trace_links_pkey PRIMARY KEY (id);


--
-- Name: trace_links trace_links_source_node_id_target_node_id_link_type_key; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_links
    ADD CONSTRAINT trace_links_source_node_id_target_node_id_link_type_key UNIQUE (source_node_id, target_node_id, link_type);


--
-- Name: trace_nodes trace_nodes_pkey; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_nodes
    ADD CONSTRAINT trace_nodes_pkey PRIMARY KEY (id);


--
-- Name: trace_nodes uq_trace_nodes_project_external; Type: CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_nodes
    ADD CONSTRAINT uq_trace_nodes_project_external UNIQUE (project_id, external_source, external_ref);


--
-- Name: idx_basis_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_basis_project ON mbsetest.project_basis_files USING btree (project_id);


--
-- Name: idx_devices_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_devices_project ON mbsetest.project_devices USING btree (project_id);


--
-- Name: idx_dte_template; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_dte_template ON mbsetest.document_template_edges USING btree (template_id);


--
-- Name: idx_dtn_template; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_dtn_template ON mbsetest.document_template_nodes USING btree (template_id);


--
-- Name: idx_env_items_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_env_items_project ON mbsetest.project_environment_items USING btree (project_id);


--
-- Name: idx_exec_inst_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_exec_inst_project ON mbsetest.execution_instances USING btree (project_id);


--
-- Name: idx_exec_inst_template; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_exec_inst_template ON mbsetest.execution_instances USING btree (template_id);


--
-- Name: idx_exec_items_instance; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_exec_items_instance ON mbsetest.execution_items USING btree (instance_id);


--
-- Name: idx_ledger_fields_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_ledger_fields_project ON mbsetest.project_ledger_fields USING btree (project_id);


--
-- Name: idx_parties_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_parties_project ON mbsetest.project_parties USING btree (project_id);


--
-- Name: idx_pfv_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_pfv_project ON mbsetest.project_field_values USING btree (project_id);


--
-- Name: idx_ptr_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_ptr_project ON mbsetest.project_table_rows USING btree (project_id);


--
-- Name: idx_schedule_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_schedule_project ON mbsetest.project_test_schedule USING btree (project_id);


--
-- Name: idx_scheme_cols_table; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_scheme_cols_table ON mbsetest.scheme_table_columns USING btree (table_id);


--
-- Name: idx_scheme_fields_scheme; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_scheme_fields_scheme ON mbsetest.scheme_fields USING btree (scheme_id);


--
-- Name: idx_scheme_tables_scheme; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_scheme_tables_scheme ON mbsetest.scheme_tables USING btree (scheme_id);


--
-- Name: idx_trace_baselines_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_baselines_project ON mbsetest.trace_baselines USING btree (project_id);


--
-- Name: idx_trace_links_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_links_project ON mbsetest.trace_links USING btree (project_id);


--
-- Name: idx_trace_links_source; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_links_source ON mbsetest.trace_links USING btree (source_node_id);


--
-- Name: idx_trace_links_target; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_links_target ON mbsetest.trace_links USING btree (target_node_id);


--
-- Name: idx_trace_nodes_kind; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_nodes_kind ON mbsetest.trace_nodes USING btree (kind);


--
-- Name: idx_trace_nodes_kind_sort; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_nodes_kind_sort ON mbsetest.trace_nodes USING btree (kind, sort_key);


--
-- Name: idx_trace_nodes_module; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_nodes_module ON mbsetest.trace_nodes USING btree (module);


--
-- Name: idx_trace_nodes_project; Type: INDEX; Schema: mbsetest; Owner: -
--

CREATE INDEX idx_trace_nodes_project ON mbsetest.trace_nodes USING btree (project_id);


--
-- Name: document_template_edges document_template_edges_source_node_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_edges
    ADD CONSTRAINT document_template_edges_source_node_id_fkey FOREIGN KEY (source_node_id) REFERENCES mbsetest.document_template_nodes(id) ON DELETE CASCADE;


--
-- Name: document_template_edges document_template_edges_target_node_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_edges
    ADD CONSTRAINT document_template_edges_target_node_id_fkey FOREIGN KEY (target_node_id) REFERENCES mbsetest.document_template_nodes(id) ON DELETE CASCADE;


--
-- Name: document_template_edges document_template_edges_template_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_edges
    ADD CONSTRAINT document_template_edges_template_id_fkey FOREIGN KEY (template_id) REFERENCES mbsetest.document_templates(id) ON DELETE CASCADE;


--
-- Name: document_template_nodes document_template_nodes_template_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.document_template_nodes
    ADD CONSTRAINT document_template_nodes_template_id_fkey FOREIGN KEY (template_id) REFERENCES mbsetest.document_templates(id) ON DELETE CASCADE;


--
-- Name: execution_instances execution_instances_template_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.execution_instances
    ADD CONSTRAINT execution_instances_template_id_fkey FOREIGN KEY (template_id) REFERENCES mbsetest.document_templates(id);


--
-- Name: execution_items execution_items_instance_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.execution_items
    ADD CONSTRAINT execution_items_instance_id_fkey FOREIGN KEY (instance_id) REFERENCES mbsetest.execution_instances(id) ON DELETE CASCADE;


--
-- Name: execution_items execution_items_template_node_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.execution_items
    ADD CONSTRAINT execution_items_template_node_id_fkey FOREIGN KEY (template_node_id) REFERENCES mbsetest.document_template_nodes(id);


--
-- Name: execution_instances fk_exec_inst_project; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.execution_instances
    ADD CONSTRAINT fk_exec_inst_project FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: projects fk_projects_scheme; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.projects
    ADD CONSTRAINT fk_projects_scheme FOREIGN KEY (scheme_id) REFERENCES mbsetest.schemes(id);


--
-- Name: trace_baselines fk_trace_baselines_project; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_baselines
    ADD CONSTRAINT fk_trace_baselines_project FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: trace_links fk_trace_links_project; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_links
    ADD CONSTRAINT fk_trace_links_project FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: trace_nodes fk_trace_nodes_project; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_nodes
    ADD CONSTRAINT fk_trace_nodes_project FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_basis_files project_basis_files_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_basis_files
    ADD CONSTRAINT project_basis_files_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_devices project_devices_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_devices
    ADD CONSTRAINT project_devices_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_environment_items project_environment_items_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_environment_items
    ADD CONSTRAINT project_environment_items_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_field_values project_field_values_field_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_field_values
    ADD CONSTRAINT project_field_values_field_id_fkey FOREIGN KEY (field_id) REFERENCES mbsetest.scheme_fields(id) ON DELETE CASCADE;


--
-- Name: project_field_values project_field_values_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_field_values
    ADD CONSTRAINT project_field_values_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_ledger_fields project_ledger_fields_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_ledger_fields
    ADD CONSTRAINT project_ledger_fields_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_parties project_parties_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_parties
    ADD CONSTRAINT project_parties_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_table_rows project_table_rows_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_table_rows
    ADD CONSTRAINT project_table_rows_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: project_table_rows project_table_rows_table_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_table_rows
    ADD CONSTRAINT project_table_rows_table_id_fkey FOREIGN KEY (table_id) REFERENCES mbsetest.scheme_tables(id) ON DELETE CASCADE;


--
-- Name: project_test_schedule project_test_schedule_project_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.project_test_schedule
    ADD CONSTRAINT project_test_schedule_project_id_fkey FOREIGN KEY (project_id) REFERENCES mbsetest.projects(id) ON DELETE CASCADE;


--
-- Name: scheme_fields scheme_fields_scheme_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_fields
    ADD CONSTRAINT scheme_fields_scheme_id_fkey FOREIGN KEY (scheme_id) REFERENCES mbsetest.schemes(id) ON DELETE CASCADE;


--
-- Name: scheme_table_columns scheme_table_columns_table_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_table_columns
    ADD CONSTRAINT scheme_table_columns_table_id_fkey FOREIGN KEY (table_id) REFERENCES mbsetest.scheme_tables(id) ON DELETE CASCADE;


--
-- Name: scheme_tables scheme_tables_scheme_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.scheme_tables
    ADD CONSTRAINT scheme_tables_scheme_id_fkey FOREIGN KEY (scheme_id) REFERENCES mbsetest.schemes(id) ON DELETE CASCADE;


--
-- Name: trace_links trace_links_source_node_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_links
    ADD CONSTRAINT trace_links_source_node_id_fkey FOREIGN KEY (source_node_id) REFERENCES mbsetest.trace_nodes(id) ON DELETE CASCADE;


--
-- Name: trace_links trace_links_target_node_id_fkey; Type: FK CONSTRAINT; Schema: mbsetest; Owner: -
--

ALTER TABLE ONLY mbsetest.trace_links
    ADD CONSTRAINT trace_links_target_node_id_fkey FOREIGN KEY (target_node_id) REFERENCES mbsetest.trace_nodes(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict gTchnenc41teAuiXDveeVaZN2xJG11pUf7x4KnTyyu6hJJ66x1cRhVQOGBtog3l

