## DAG execution example

- Orchestrator starts DAG → spawns worker VM inside private VPC.
- Worker boots → requests temporary role from metadata service / broker.
- Role credentials granted → worker runs DAG nodes.
- DAG node completes → results stored in private storage.
- DAG completes → worker shuts down → temporary credentials automatically expire.
- Next DAG → repeat process.

Sensitive data remains in the VPC and no permanent secrets are exposed.

## Job Claim Logic

```text
loop:
    begin tx
        claim job_run
    commit tx

    execute job outside transaction

    mark complete/fail
```
