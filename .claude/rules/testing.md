# Unit Tests and Quality Control

- Always check that the unit tests pass.
- Always test the code, make sure tests work after the changes.
- Always write tests that check the algorithms, or meaningful edge cases. Never write tests that check things that can be handled by types instead.
- If some piece of code can be handled by proper types, use types instead. Write tests as a last resort.
- In unit tests, make sure there is always just a single correct way to do a specific thing. Never accept fuzzy inputs from end users.
- When working on tests, if you notice that the tested code can be better, you can suggest changes.
