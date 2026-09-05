#!/usr/bin/env node
// Minimal LSP server fixture for batch N3 production-path tests.
// Speaks the real LSP wire protocol over stdio: initialize handshake,
// textDocument/didOpen tracking, and definition/references answered from a
// symbol table built by scanning the workspace for `pub fn NAME(` patterns.
// It exists so the daemon's LSP enrichment can be tested against a REAL
// language-server process (sandboxed spawn + full JSON-RPC framing) on
// machines where rust-analyzer is not installed — no mocks of the client.

const fs = require("fs");
const path = require("path");

let rootUri = null;
const documents = new Map(); // uri -> text
let nextId = 1000;

function write(msg) {
  const body = Buffer.from(JSON.stringify(msg));
  process.stdout.write(
    `Content-Length: ${body.length}\r\n\r\n` + body.toString("utf8")
  );
}

function respond(id, result) {
  write({ jsonrpc: "2.0", id, result });
}

// Build a symbol table: symbol name -> [uri, line] for every `pub fn NAME(`
// (plus `fn NAME(`) found in .rs/.ts/.py/.go files under root.
function symbolTable() {
  const root = rootUri ? decodeURIComponent(rootUri.replace("file://", "")) : ".";
  const table = new Map();
  const walk = (dir, depth) => {
    if (depth > 6) return;
    let entries;
    try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { return; }
    for (const entry of entries) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === "target" || entry.name === "node_modules" || entry.name.startsWith(".")) continue;
        walk(full, depth + 1);
      } else if (/\.(rs|ts|tsx|js|py|go)$/.test(entry.name)) {
        let text;
        try { text = fs.readFileSync(full, "utf8"); } catch { continue; }
        text.split("\n").forEach((line, idx) => {
          const m = line.match(/\b(?:pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/) ||
                    line.match(/\bdef\s+([A-Za-z_][A-Za-z0-9_]*)\(/) ||
                    line.match(/\bfunc\s+([A-Za-z_][A-Za-z0-9_]*)\(/);
          if (m) {
            if (!table.has(m[1])) table.set(m[1], []);
            table.get(m[1]).push({ uri: `file://${full}`, line: idx });
          }
        });
      }
    }
  };
  walk(root, 0);
  return table;
}

function findWordAtPosition(uri, text, line, character) {
  const lines = text.split("\n");
  if (line >= lines.length) return null;
  const l = lines[line];
  // expand to identifier boundaries
  let start = character;
  let end = character;
  const isWord = (c) => /[A-Za-z0-9_]/.test(c);
  while (start > 0 && isWord(l[start - 1])) start--;
  while (end < l.length && isWord(l[end])) end++;
  const word = l.slice(start, end);
  return word.length > 2 ? word : null;
}

// The symbol table is built LAZILY after initialize delivers rootUri —
// building it at startup would walk the wrong directory (rootUri is not
// yet known) and hand back locations from unrelated files.
let table = null;

let buffer = Buffer.alloc(0);
process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd === -1) return;
    const header = buffer.slice(0, headerEnd).toString("utf8");
    const m = header.match(/Content-Length:\s*(\d+)/);
    if (!m) { buffer = Buffer.alloc(0); return; }
    const length = parseInt(m[1], 10);
    if (buffer.length < headerEnd + 4 + length) return;
    const body = buffer.slice(headerEnd + 4, headerEnd + 4 + length).toString("utf8");
    buffer = buffer.slice(headerEnd + 4 + length);
    let msg;
    try { msg = JSON.parse(body); } catch { continue; }
    handleMessage(msg);
  }
});

function handleMessage(msg) {
  if (msg.method === "initialize") {
    rootUri = msg.params && msg.params.rootUri;
    write({
      jsonrpc: "2.0",
      id: msg.id,
      result: {
        capabilities: {
          textDocumentSync: 1,
          definitionProvider: true,
          referencesProvider: true,
          renameProvider: { prepareSupport: true },
        },
        serverInfo: { name: "ac-lsp-fixture", version: "0.1.0" },
      },
    });
  } else if (msg.method === "initialized") {
    // notification, no response
  } else if (msg.method === "shutdown") {
    respond(msg.id, null);
  } else if (msg.method === "exit") {
    process.exit(0);
  } else if (msg.method === "textDocument/didOpen") {
    const td = msg.params.textDocument;
    documents.set(td.uri, td.text);
  } else if (msg.method === "textDocument/definition") {
    if (!table) table = symbolTable();
    const uri = msg.params.textDocument.uri;
    const text = documents.get(uri) || fs.readFileSync(decodeURIComponent(uri.replace("file://", "")), "utf8");
    const word = findWordAtPosition(uri, text, msg.params.position.line, msg.params.position.character);
    const hits = (table.get(word) || []);
    respond(msg.id, hits.map((h) => ({
      uri: h.uri,
      range: { start: { line: h.line, character: 0 }, end: { line: h.line, character: 1 } },
    })));
  } else if (msg.method === "textDocument/references") {
    if (!table) table = symbolTable();
    const uri = msg.params.textDocument.uri;
    const text = documents.get(uri) || fs.readFileSync(decodeURIComponent(uri.replace("file://", "")), "utf8");
    const word = findWordAtPosition(uri, text, msg.params.position.line, msg.params.position.character);
    const hits = (table.get(word) || []);
    respond(msg.id, hits.map((h) => ({
      uri: h.uri,
      range: { start: { line: h.line, character: 0 }, end: { line: h.line, character: 1 } },
    })));
  } else if (msg.method === "textDocument/rename") {
    const uri = msg.params.textDocument.uri;
    const text = documents.get(uri) || fs.readFileSync(decodeURIComponent(uri.replace("file://", "")), "utf8");
    const word = findWordAtPosition(uri, text, msg.params.position.line, msg.params.position.character);
    const edits = [];
    for (const [docUri, docText] of documents) {
      docText.split("\n").forEach((line, idx) => {
        let col;
        let from = 0;
        while ((col = line.indexOf(word, from)) !== -1) {
          edits.push(docUri, idx, col, word);
          from = col + word.length;
        }
      });
    }
    respond(msg.id, null); // simple stub: rename tested elsewhere
  } else if (msg.id !== undefined) {
    respond(msg.id, null);
  }
}
