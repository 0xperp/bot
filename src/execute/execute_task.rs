use {
    crate::{
        replicate::replicate_task,
        utils::{new_rpc_client, sign_and_submit},
    },
    anchor_lang::prelude::{AccountMeta, Pubkey},
    cronos_sdk::account::*,
    std::thread,
};

// TODO make cache time configurable thru CLI params
//
// WARNING
// Lower cache times are useful when you want to run a bot with faster execution behaviors.
// In many cases, this strategy isn't always efficient. Lower cache times lead a bot
// to spend more on compute budgets and "compete with itself" for RPC threads.

#[cached::proc_macro::cached(size = 1_000_000, time = 4)]
pub fn execute_task(pubkey: Pubkey, daemon: Pubkey) {
    thread::spawn(move || {
        let client = new_rpc_client();
        let data = client.get_account_data(&pubkey).unwrap();
        let task = Task::try_from(data).unwrap();
        match task.status {
            TaskStatus::Cancelled | TaskStatus::Done => {
                replicate_task(pubkey, task);
                return;
            }
            TaskStatus::Queued => {
                let config = Config::pda().0;
                let fee = Fee::pda(daemon).0;
                let mut ix = cronos_sdk::instruction::task_execute(
                    config,
                    daemon,
                    fee,
                    pubkey,
                    client.payer_pubkey(),
                );
                for acc in task.ix.accounts {
                    match acc.is_writable {
                        true => ix.accounts.push(AccountMeta::new(acc.pubkey, false)),
                        false => ix
                            .accounts
                            .push(AccountMeta::new_readonly(acc.pubkey, false)),
                    }
                }
                ix.accounts
                    .push(AccountMeta::new_readonly(task.ix.program_id, false));
                sign_and_submit(
                    client,
                    &[ix],
                    format!("Executing task: {} {}", pubkey, daemon).as_str(),
                );
            }
        }
    });
}
