-- Migration 001: Initial Schema
-- koompi-orch database tables, relations, and indexes

-- Migration tracking
DEFINE TABLE migration SCHEMAFULL;
DEFINE FIELD version ON migration TYPE int;
DEFINE FIELD name ON migration TYPE string;
DEFINE FIELD applied_at ON migration TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_migration_version ON migration FIELDS version UNIQUE;

-- Repos
DEFINE TABLE repo SCHEMAFULL;
DEFINE FIELD path ON repo TYPE string;
DEFINE FIELD name ON repo TYPE string;
DEFINE FIELD remote_url ON repo TYPE option<string>;
DEFINE FIELD added_at ON repo TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_repo_path ON repo FIELDS path UNIQUE;

-- Workspaces
DEFINE TABLE workspace SCHEMAFULL;
DEFINE FIELD name ON workspace TYPE string;
DEFINE FIELD branch ON workspace TYPE string;
DEFINE FIELD worktree_path ON workspace TYPE string;
DEFINE FIELD status ON workspace TYPE string
    ASSERT $value IN ['backlog','active','review','done','failed'];
DEFINE FIELD locked_by ON workspace TYPE option<record<session>>;
DEFINE FIELD created_at ON workspace TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON workspace TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_workspace_status ON workspace FIELDS status;

-- Relation: workspace belongs to repo
DEFINE TABLE belongs_to SCHEMAFULL TYPE RELATION IN workspace OUT repo;

-- Agent sessions
DEFINE TABLE session SCHEMAFULL;
DEFINE FIELD agent_type ON session TYPE string;
DEFINE FIELD model ON session TYPE option<string>;
DEFINE FIELD pid ON session TYPE option<int>;
DEFINE FIELD role_preset ON session TYPE option<string>;
DEFINE FIELD status ON session TYPE string
    ASSERT $value IN ['running','paused','completed','crashed'];
DEFINE FIELD started_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD ended_at ON session TYPE option<datetime>;
DEFINE FIELD config ON session TYPE object;
DEFINE INDEX idx_session_status ON session FIELDS status;

-- Relation: session runs in workspace
DEFINE TABLE runs_in SCHEMAFULL TYPE RELATION IN session OUT workspace;

-- Pipeline definitions
DEFINE TABLE pipeline SCHEMAFULL;
DEFINE FIELD name ON pipeline TYPE string;
DEFINE FIELD steps ON pipeline TYPE array<object>;
DEFINE FIELD created_at ON pipeline TYPE datetime DEFAULT time::now();

-- Pipeline execution instances
DEFINE TABLE pipeline_run SCHEMAFULL;
DEFINE FIELD current_step ON pipeline_run TYPE int DEFAULT 0;
DEFINE FIELD status ON pipeline_run TYPE string
    ASSERT $value IN ['running','paused','completed','failed'];
DEFINE FIELD started_at ON pipeline_run TYPE datetime DEFAULT time::now();
DEFINE FIELD ended_at ON pipeline_run TYPE option<datetime>;
DEFINE TABLE instance_of SCHEMAFULL TYPE RELATION IN pipeline_run OUT pipeline;
DEFINE TABLE executes_in SCHEMAFULL TYPE RELATION IN pipeline_run OUT workspace;

-- Relation: session hands off to session
DEFINE TABLE hands_off_to SCHEMAFULL TYPE RELATION IN session OUT session;
DEFINE FIELD handoff_type ON hands_off_to TYPE string
    ASSERT $value IN ['summary','full_log','diff_only'];
DEFINE FIELD output_summary ON hands_off_to TYPE option<string>;
DEFINE FIELD context_file ON hands_off_to TYPE option<string>;
DEFINE FIELD handoff_at ON hands_off_to TYPE datetime DEFAULT time::now();

-- Checkpoints
DEFINE TABLE checkpoint SCHEMAFULL;
DEFINE FIELD commit_sha ON checkpoint TYPE string;
DEFINE FIELD turn_number ON checkpoint TYPE int;
DEFINE FIELD description ON checkpoint TYPE option<string>;
DEFINE FIELD created_at ON checkpoint TYPE datetime DEFAULT time::now();
DEFINE TABLE checkpoint_of SCHEMAFULL TYPE RELATION IN checkpoint OUT workspace;

-- Metrics (append-only)
DEFINE TABLE metric SCHEMAFULL;
DEFINE FIELD tokens_in ON metric TYPE int DEFAULT 0;
DEFINE FIELD tokens_out ON metric TYPE int DEFAULT 0;
DEFINE FIELD cost_usd ON metric TYPE float DEFAULT 0.0;
DEFINE FIELD duration_ms ON metric TYPE int DEFAULT 0;
DEFINE FIELD turn_number ON metric TYPE int DEFAULT 0;
DEFINE FIELD recorded_at ON metric TYPE datetime DEFAULT time::now();
DEFINE TABLE metric_for SCHEMAFULL TYPE RELATION IN metric OUT session;

-- Agent templates
DEFINE TABLE agent_template SCHEMAFULL;
DEFINE FIELD name ON agent_template TYPE string;
DEFINE FIELD command ON agent_template TYPE string;
DEFINE FIELD default_args ON agent_template TYPE array<string>;
DEFINE FIELD env ON agent_template TYPE option<object>;
DEFINE FIELD input_mode ON agent_template TYPE string
    ASSERT $value IN ['pty_stdin','flag_message','file_prompt'];
DEFINE FIELD output_mode ON agent_template TYPE string
    ASSERT $value IN ['json_stream','text_markers','raw_pty'];
DEFINE FIELD resume_support ON agent_template TYPE bool DEFAULT false;
DEFINE FIELD builtin ON agent_template TYPE bool DEFAULT false;
DEFINE INDEX idx_template_name ON agent_template FIELDS name UNIQUE;

-- Role presets
DEFINE TABLE role_preset SCHEMAFULL;
DEFINE FIELD name ON role_preset TYPE string;
DEFINE FIELD system_prompt ON role_preset TYPE string;
DEFINE FIELD description ON role_preset TYPE string;
DEFINE FIELD injection_method ON role_preset TYPE string
    ASSERT $value IN ['flag','env_var','config_file','first_message'];
DEFINE FIELD builtin ON role_preset TYPE bool DEFAULT false;
DEFINE INDEX idx_preset_name ON role_preset FIELDS name UNIQUE;
