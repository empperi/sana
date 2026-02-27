# Operating principles (non-negotiable)

1. **Smallest change that works**: Minimize blast radius; don't refactor adjacent code unless it meaningfully reduces 
   risk or complexity.
2. **Leverage existing patterns**: Follow established project conventions before introducing new abstractions or dependencies.
3. **Prove it works**: "Seems right" is not done. Validate with tests/build/lint and/or a reliable manual repro. 
4. **Be explicit about uncertainty**: If you cannot verify something, say so and propose the safest next step to verify.

# Coding style instructions

1. Always prefer functional style over imperative style
2. Prefer immutable data over mutable unless it causes significant performance hit. If you do, ensure mutability is
   encapsulated in such a way that it cannot leak
3. Use TDD and implement tests for everything
4. With tests prefer unit tests over integration tests
5. With Rust unit tests, create those into separate test files, do not write them into actual implementation file as mod
6. With tests always write tests first in the file and fixtures and helper functions after the tests
7. Do not try to test private functions, use pure functions and functional style instead so testing public functions
   makes testing easy without excessive state setup
8. Keep lines at maximum of 120 characters unless it would make code less readable to split a line
