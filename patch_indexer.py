"""
This script is a build-time monkey-patch for the 'code-index-mcp' library.

PURPOSE:
The 'code-index-mcp' library does not natively support deep structural indexing for Rust 
(it treats .rs files as plain text fallback). This script injects a custom 
'RustParsingStrategy' that uses tree-sitter-rust to extract functions, structs, 
enums, and traits.

WHY THIS CODE EXISTS:
1.  Persistent Deep Indexing: By applying this patch during the Docker build phase 
    (see Dockerfile.mcp-index), deep Rust indexing becomes baked into the image.
2.  Environment Portability: This ensures the indexer works identically on any 
    machine (macOS, Windows, etc.) without requiring manual library modifications.
3.  Tooling Workaround: This avoids issues where the LLM might lack structural 
    awareness of the Rust codebase because the underlying indexer is incomplete.

HOW IT WORKS:
- It creates a new 'rust_strategy.py' within the installed library's directory.
- It modifies the 'StrategyFactory' in the library to register and use this new strategy.
"""

import os
import textwrap
import code_index_mcp.indexing.strategies as strategies

strategy_dir = os.path.dirname(strategies.__file__)

# 1. Create rust_strategy.py
rust_strategy_path = os.path.join(strategy_dir, "rust_strategy.py")
rust_code = textwrap.dedent("""\
    from typing import Dict, List, Tuple, Optional
    import tree_sitter
    import tree_sitter_rust
    from .base_strategy import ParsingStrategy
    from ..models import SymbolInfo, FileInfo

    class RustParsingStrategy(ParsingStrategy):
        def __init__(self):
            self.rust_language = tree_sitter.Language(tree_sitter_rust.language())

        def get_language_name(self) -> str:
            return "rust"

        def get_supported_extensions(self) -> List[str]:
            return [".rs"]

        def parse_file(self, file_path: str, content: str) -> Tuple[Dict[str, SymbolInfo], FileInfo]:
            parser = tree_sitter.Parser(self.rust_language)
            tree = parser.parse(content.encode('utf8'))
            
            symbols: Dict[str, SymbolInfo] = {}
            functions: List[str] = []
            classes: List[str] = []
            imports: List[str] = []
            
            self._traverse_node(tree.root_node, content, file_path, symbols, functions, classes, imports)
            
            file_info = FileInfo(
                language="rust",
                line_count=len(content.splitlines()),
                symbols={"functions": functions, "classes": classes},
                imports=imports
            )
            
            return symbols, file_info

        def _traverse_node(self, node, content, file_path, symbols, functions, classes, imports, parent_class=None):
            node_type = node.type
            
            # Imports (use statements)
            if node_type == 'use_declaration':
                import_text = content[node.start_byte:node.end_byte].strip()
                if import_text not in imports:
                    imports.append(import_text)
                return # Don't recurse into use declarations

            # Function definitions (top-level or in impl)
            if node_type == 'function_item':
                name_node = node.child_by_field_name('name')
                if name_node:
                    name = content[name_node.start_byte:name_node.end_byte]
                    full_name = f"{parent_class}.{name}" if parent_class else name
                    symbol_type = "method" if parent_class else "function"
                    
                    symbol_id = self._create_symbol_id(file_path, full_name)
                    symbols[symbol_id] = SymbolInfo(
                        type=symbol_type,
                        file=file_path,
                        line=node.start_point[0] + 1,
                        end_line=node.end_point[0] + 1,
                        signature=name
                    )
                    functions.append(full_name)
            
            # Struct definitions
            elif node_type == 'struct_item':
                name_node = node.child_by_field_name('name')
                if name_node:
                    name = content[name_node.start_byte:name_node.end_byte]
                    symbol_id = self._create_symbol_id(file_path, name)
                    symbols[symbol_id] = SymbolInfo(
                        type="struct",
                        file=file_path,
                        line=node.start_point[0] + 1,
                        end_line=node.end_point[0] + 1,
                        signature=name
                    )
                    classes.append(name)

            # Enum definitions
            elif node_type == 'enum_item':
                name_node = node.child_by_field_name('name')
                if name_node:
                    name = content[name_node.start_byte:name_node.end_byte]
                    symbol_id = self._create_symbol_id(file_path, name)
                    symbols[symbol_id] = SymbolInfo(
                        type="enum",
                        file=file_path,
                        line=node.start_point[0] + 1,
                        end_line=node.end_point[0] + 1,
                        signature=name
                    )
                    classes.append(name)

            # Trait definitions
            elif node_type == 'trait_item':
                name_node = node.child_by_field_name('name')
                if name_node:
                    name = content[name_node.start_byte:name_node.end_byte]
                    symbol_id = self._create_symbol_id(file_path, name)
                    symbols[symbol_id] = SymbolInfo(
                        type="trait",
                        file=file_path,
                        line=node.start_point[0] + 1,
                        end_line=node.end_point[0] + 1,
                        signature=name
                    )
                    classes.append(name)

            # Implementation blocks
            elif node_type == 'impl_item':
                # Attempt to find the type being implemented
                type_node = node.child_by_field_name('type')
                current_parent = None
                if type_node:
                    current_parent = content[type_node.start_byte:type_node.end_byte]
                
                # Trait impl? (impl Trait for Type)
                trait_node = node.child_by_field_name('trait')
                if trait_node and type_node:
                    trait_name = content[trait_node.start_byte:trait_node.end_byte]
                    type_name = content[type_node.start_byte:type_node.end_byte]
                    current_parent = f"({trait_name} for {type_name})"

                for child in node.children:
                    self._traverse_node(child, content, file_path, symbols, functions, classes, imports, parent_class=current_parent)
                return

            # Recurse into children
            for child in node.children:
                self._traverse_node(child, content, file_path, symbols, functions, classes, imports, parent_class=parent_class)
    """)

with open(rust_strategy_path, "w") as f:
    f.write(rust_code)

# 2. Patch strategy_factory.py
factory_path = os.path.join(strategy_dir, "strategy_factory.py")
with open(factory_path, "r") as f:
    lines = f.readlines()

if not any("rust_strategy" in line for line in lines):
    new_lines = []
    for line in lines:
        new_lines.append(line)
        if "from .fallback_strategy import FallbackParsingStrategy" in line:
            new_lines.append("from .rust_strategy import RustParsingStrategy\n")
        elif "objc_strategy = ObjectiveCParsingStrategy()" in line:
            new_lines.append("                # Rust\n")
            new_lines.append("                rust_strategy = RustParsingStrategy()\n")
            new_lines.append("                for ext in rust_strategy.get_supported_extensions():\n")
            new_lines.append("                    self._strategies[ext] = rust_strategy\n")
    with open(factory_path, "w") as f:
        f.writelines(new_lines)
