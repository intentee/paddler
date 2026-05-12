# Unit Tests and Quality Control

- Always check that the unit tests pass.
- Always test the code, make sure tests work after the changes.
- Always write tests that check the algorithms, or meaningful edge cases. Never write tests that check things that can be handled by types instead.
- If some piece of code can be handled by proper types, use types instead. Write tests as a last resort.
- In unit tests, make sure there is always just a single correct way to do a specific thing. Never accept fuzzy inputs from end users.
- When working on tests, if you notice that the tested code can be better, you can suggest changes.
- Maintain 100% test coverage across the codebase. No file, branch, or line may be excluded from coverage reports.
- Reach 100% coverage with the minimum number of tests. Each test must cover a unique code path, behavior, or edge case that no other test already covers.
- If two tests cover overlapping paths, remove the weaker one. Redundant tests waste maintenance effort without improving correctness signal.
- Tests must exercise actual functionality and observable behavior. Never write a test purely to hit lines for the sake of coverage.
- Design tests deliberately before writing them. Identify the feature or branch under test, then write the smallest test that verifies it.
- Coverage gaps signal missing tests, never permission to exclude files. Write the test instead of suppressing the gap.
