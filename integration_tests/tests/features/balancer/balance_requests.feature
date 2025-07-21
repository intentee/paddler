Feature: Balance llama.cpp requests

    @serial
    Scenario: There are no agents attached
        Given buffered requests timeout after 0 milliseconds
        Given balancer is running
        When request "foo" is sent to "/chat/completions"
        Then "foo" response code is 504

    @serial
    Scenario: There is one agent attached
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 4 slots)
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        When request "foo" is sent to "/chat/completions"
        Then "foo" response code is 200
        Then "foo" request landed in "llama-1"

    @serial
    Scenario: There are multiple agents attached
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 4 slots)
        Given llama.cpp server "llama-2" is running (has 4 slots)
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        Given agent "agent-2" is running (observes "llama-2")
        Given agent "agent-2" is registered
        When multiple requests are sent to "/chat/completions"
            | req-1 |
            | req-2 |
        Then "req-1" response code is 200
        Then "req-1" request landed in "llama-1"
        Then "req-2" response code is 200
        Then "req-2" request landed in "llama-2"

    @serial
    Scenario: More requests than slots available, buffering enabled
        Given buffered requests timeout after 0 milliseconds
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 1 slot)
        Given llama.cpp server "llama-2" is running (has 1 slot)
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        Given agent "agent-2" is running (observes "llama-2")
        Given agent "agent-2" is registered
        When multiple requests are sent to "/chat/completions"
            | req-1 |
            | req-2 |
            | req-3 |
        Then "req-1" response code is 200
        Then "req-1" request landed in "llama-1"
        Then "req-2" response code is 200
        Then "req-2" request landed in "llama-2"
        Then "req-3" response code is 504

    @serial
    Scenario: More requests than slots available, buffering time adequate
        Given buffered requests timeout after 2000 milliseconds
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 1 slot)
        Given llama.cpp server "llama-2" is running (has 1 slot)
        Given agent monitors llama.cpp every 50 milliseconds
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        Given agent "agent-2" is running (observes "llama-2")
        Given agent "agent-2" is registered
        When multiple requests are sent to "/chat/completions"
            | req-1 |
            | req-2 |
            | req-3 |
        Then "req-1" response code is 200
        Then "req-1" request landed in "llama-1"
        Then "req-2" response code is 200
        Then "req-2" request landed in "llama-2"
        Then "req-3" response code is 200
        Then "req-3" request landed in "llama-1"

    @serial
    Scenario: More requests than slots available, buffering disabled
        Given request buffering is disabled
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 1 slot)
        Given llama.cpp server "llama-2" is running (has 1 slot)
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        Given agent "agent-2" is running (observes "llama-2")
        Given agent "agent-2" is registered
        When multiple requests are sent to "/chat/completions"
            | req-1 |
            | req-2 |
            | req-3 |
        Then "req-1" response code is 200
        Then "req-1" request landed in "llama-1"
        Then "req-2" response code is 200
        Then "req-2" request landed in "llama-2"
        Then "req-3" response code is 429

    @serial
    Scenario: Balancing between multiple slots and agents
        Given balancer is running
        Given llama.cpp server "llama-1" is running (has 2 slots)
        Given llama.cpp server "llama-2" is running (has 2 slots)
        Given agent "agent-1" is running (observes "llama-1")
        Given agent "agent-1" is registered
        Given agent "agent-2" is running (observes "llama-2")
        Given agent "agent-2" is registered
        When multiple requests are sent to "/chat/completions"
            | req-1 |
            | req-2 |
            | req-3 |
            | req-4 |
            | req-5 |
        Then "req-1" response code is 200
        Then "req-1" request landed in "llama-1"
        Then "req-2" response code is 200
        Then "req-2" request landed in "llama-2"
        Then "req-3" response code is 200
        Then "req-3" request landed in "llama-1"
        Then "req-4" response code is 200
        Then "req-4" request landed in "llama-2"
        Then "req-5" response code is 200
        Then "req-5" request landed in "llama-1"