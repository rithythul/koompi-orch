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
    <div className="flex flex-col gap-2">
      {templates.map((template) => (
        <div
          key={template.id}
          className="px-4 py-3 bg-gray-800/50 border border-gray-700 rounded-lg"
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
                className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              />
              <input
                type="text"
                value={editForm.command ?? ""}
                onChange={(e) =>
                  setEditForm({ ...editForm, command: e.target.value })
                }
                placeholder="Command (e.g., claude)"
                className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
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
                className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-sm text-gray-200 focus:outline-none focus:border-blue-500"
              />
              <div className="flex gap-2">
                <select
                  value={editForm.inputMode ?? "pty_stdin"}
                  onChange={(e) =>
                    setEditForm({ ...editForm, inputMode: e.target.value })
                  }
                  className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200"
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
                  className="bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200"
                >
                  <option value="json_stream">JSON stream</option>
                  <option value="text_markers">Text markers</option>
                  <option value="raw_pty">Raw PTY</option>
                </select>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={handleSave}
                  className="px-3 py-1 text-xs text-green-400 hover:text-green-300 border border-green-800 rounded"
                >
                  Save
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingId(null);
                    setEditForm({});
                  }}
                  className="px-3 py-1 text-xs text-gray-500 hover:text-gray-300"
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-gray-200">
                  {template.name}
                </span>
                {template.builtIn && (
                  <span className="text-[10px] font-semibold text-blue-400 bg-blue-900/30 px-1.5 py-0.5 rounded">
                    Built-in
                  </span>
                )}
                <span className="text-xs text-gray-500 font-mono">
                  {template.command}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => startEdit(template)}
                  className="px-2 py-1 text-xs text-blue-400 hover:text-blue-300"
                >
                  Edit
                </button>
                {!template.builtIn && (
                  <button
                    type="button"
                    onClick={() => onDelete(template.id)}
                    className="px-2 py-1 text-xs text-red-400 hover:text-red-300"
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
