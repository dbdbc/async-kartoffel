-- Configures the rust-analyzer lsp to work in both workspaces, the outer one architecture-agnostic
-- one (Cargo.toml), and the inner target-specific one (cross/Cargo.toml)

local clients = vim.lsp.get_clients({ name = "rust_analyzer" })
if #clients == 0 then
	print("no rust_analyzer lsp is currently active, open a rust file or check :LspInfo")
end
for _, client in ipairs(clients) do
	if client.settings["rust-analyzer"] == nil then
		client.settings["rust-analyzer"] = {}
	end
	client.settings["rust-analyzer"]["linkedProjects"] = { "Cargo.toml", "cross/Cargo.toml" }

	client:notify("workspace/didChangeConfiguration", {
		settings = client.settings,
	})
end
