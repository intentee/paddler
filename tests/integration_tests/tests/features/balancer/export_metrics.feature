Feature: Expose llama.cpp metrics

    Background:
        Given balancer is running
 
    @serial
    Scenario: There is no agent attached
        Then average metrics are:
            | requests_buffered | 0 |
            | slots_idle        | 0 | 
            | slots_processing  | 0 |
            
    @serial
    Scenario: There are multiple agents attached
        Given agent monitors llama.cpp every 100 milliseconds
        Given llama.cpp completes response after 600 milliseconds
        Given llama.cpp server "llama-1" is running (has 1 slots)
        Given llama.cpp server "llama-2" is running (has 1 slots)
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
        Then exported metrics are stored
        When multiple requests are sent to "/chat/completions"
            | req-6  |
            | req-7  |
            | req-8  |
            | req-9  |
            | req-10 |
        Then exported metrics are stored
        Then average metrics are:
            | slots_idle        | 1 | 
            | slots_processing  | 1 |
            | requests_buffered | 0 |
