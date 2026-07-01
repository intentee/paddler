Feature: Monitor llama.cpp instances

    @serial
    Scenario: Agent attaches llama.cpp
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 1 slot)
        Given llama.cpp server "llama-2" is running (has 1 slot)
        Given llama.cpp server "llama-3" is running (has 1 slot)
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        Given agent "agent-2" is running (observes "llama-2")
        Given agent "agent-2" is registered
        Given agent "agent-3" is running (observes "llama-3")
        Given agent "agent-3" is registered
        Then balancer state is:
            | agent   | is_connect_error |
            | agent-1 | false            |
            | agent-2 | false            |
            | agent-3 | false            |
        When llama.cpp server "llama-1" stops running
        Then next balancer state is:
            | agent   | is_connect_error |
            | agent-1 | true             |
            | agent-2 | false            |
            | agent-3 | false            |
        When llama.cpp server "llama-3" stops running
        Then next balancer state is:
            | agent   | is_connect_error |
            | agent-1 | true             |
            | agent-2 | false            |
            | agent-3 | true             |
