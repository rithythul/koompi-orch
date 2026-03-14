import { useState } from "react";

interface AgentTemplate {
  id: string;
  name: string;
  command: string;
  args: string[];
  inputMode: string;
  outputMode: string;
  builtIn: boolean;
}

interface AgentTemplatesProps {
  templates: AgentTemplate[];
  onSave: (template: AgentTemplate) => void;
  onDelete: (id: string) => void;
}

export function AgentTemplates({
  templates,
  onSave,
  onDelete,
}: AgentTemplatesProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<Partial<AgentTemplate>>({});

  const startEdit = (template: AgentTemplate) => {
    setEditingId(template.id);
    setEditForm({ ...template });
  };

  const handleSave = () => {
    if (editingId && editForm.name && editForm.command) {
      onSave({
        id: editForm.id ?? editingId,
        name: editForm.name,
        command: editForm.command,
        args: editForm.args ?? [],
        inputMode: editForm.inputMode ?? "pty_stdin",
        outputMode: editForm.outputMode ?? "text_markers",
        builtIn: editForm.builtIn ?? false,
      });
      setEditingId(null);
      setEditForm({});
    }
  };

  return (
    <div className="flex flex-col gap-1">
      {templates.map((template) => (
        <div
          key={template.id}
          className="px-4 py-3 border-b border-border last:border-b-0"
        >
          {editingId === template.id ? (
            <div className="flex flex-col gap-2">
              <input
                type="text"
                value={editForm.name ?? ""}
                onChange={(e) =>
                  setEditForm({ ...editForm, name: e.target.value })
                }
                placeholder="Template name"
                className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[13px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
              />
              <input
                type="text"
                value={editForm.command ?? ""}
                onChange={(e) =>
                  setEditForm({ ...editForm, command: e.target.value })
                }
                placeholder="Command (e.g., claude)"
                className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[13px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
              />
              <input
                type="text"
                value={(editForm.args ?? []).join(" ")}
                onChange={(e) =>
                  setEditForm({
                    ...editForm,
                    args: e.target.value.split(" ").filter(Boolean),
                  })
                }
                placeholder="Args (space-separated)"
                className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[13px] text-text-primary placeholder:text-text-ghost focus:outline-none focus:border-accent transition-colors"
              />
              <div className="flex gap-2">
                <select
                  value={editForm.inputMode ?? "pty_stdin"}
                  onChange={(e) =>
                    setEditForm({ ...editForm, inputMode: e.target.value })
                  }
                  className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[12px] text-text-primary focus:outline-none focus:border-accent transition-colors cursor-pointer"
                >
                  <option value="pty_stdin">PTY stdin</option>
                  <option value="flag_message">Flag message</option>
                  <option value="file_prompt">File prompt</option>
                </select>
                <select
                  value={editForm.outputMode ?? "text_markers"}
                  onChange={(e) =>
                    setEditForm({ ...editForm, outputMode: e.target.value })
                  }
                  className="bg-input-bg border border-border rounded-md px-2.5 py-1.5 text-[12px] text-text-primary focus:outline-none focus:border-accent transition-colors cursor-pointer"
                >
                  <option value="json_stream">JSON stream</option>
                  <option value="text_markers">Text markers</option>
                  <option value="raw_pty">Raw PTY</option>
                </select>
              </div>
              <div className="flex gap-2 pt-1">
                <button
                  type="button"
                  onClick={handleSave}
                  className="px-3 py-1.5 text-[12px] text-success hover:text-success/80 border border-success/30 rounded-md transition-colors"
                >
                  Save
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingId(null);
                    setEditForm({});
                  }}
                  className="px-3 py-1.5 text-[12px] text-text-ghost hover:text-text-secondary transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="text-[13px] font-medium text-text-primary">
                  {template.name}
                </span>
                {template.builtIn && (
                  <span className="text-[9px] font-mono uppercase tracking-wider text-accent bg-accent-muted px-1.5 py-0.5 rounded">
                    Built-in
                  </span>
                )}
                <span className="text-[11px] text-text-ghost font-mono">
                  {template.command}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => startEdit(template)}
                  className="px-2 py-1 text-[12px] text-accent hover:text-accent-hover transition-colors"
                >
                  Edit
                </button>
                {!template.builtIn && (
                  <button
                    type="button"
                    onClick={() => onDelete(template.id)}
                    className="px-2 py-1 text-[12px] text-error hover:text-error/80 transition-colors"
                  >
                    Delete
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
