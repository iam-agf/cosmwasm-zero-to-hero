#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Ballot, Config, Poll, BALLOTS, CONFIG, POLLS};

const CONTRACT_NAME: &str = "crates.io:cw-starter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let admin = msg.admin.unwrap_or(info.sender.to_string());
    let validated_admin = deps.api.addr_validate(&admin)?;
    let config = Config {
        admin: validated_admin.clone(),
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", validated_admin.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePoll {
            poll_id,
            question,
            options,
        } => execute_create_poll(deps, env, info, poll_id, question, options),
        ExecuteMsg::Vote { poll_id, vote } => execute_vote(deps, env, info, poll_id, vote),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

fn execute_create_poll(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    poll_id: String,
    question: String,
    options: Vec<String>,
) -> Result<Response, ContractError> {
    // Restricts # of options for creating the poll
    if options.len() > 5 {
        return Err(ContractError::TooManyOptions {});
    }

    // Generates a vector for the options to make the register of votes later
    let mut opts: Vec<(String, u64)> = vec![];
    for option in options {
        opts.push((option, 0));
    }

    // Generates the poll
    let poll = Poll {
        creator: info.sender,
        question,
        options: opts,
    };

    POLLS.save(deps.storage, poll_id, &poll)?;

    Ok(Response::new())
}

fn execute_vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    poll_id: String,
    vote: String,
) -> Result<Response, ContractError> {
    let poll = POLLS.may_load(deps.storage, poll_id.clone())?;

    match poll {
        // Poll exists
        Some(mut poll) => {
            BALLOTS.update(
                deps.storage,
                (info.sender, poll_id.clone()),
                |ballot| -> StdResult<Ballot> {
                    match ballot {
                        Some(ballot) => {
                            // Check if user has already voted
                            let position_of_old_vote = poll
                                .options
                                .iter()
                                .position(|option| option.0 == ballot.option)
                                .unwrap();
                            poll.options[position_of_old_vote].1 -= 1;
                            Ok(Ballot {
                                option: vote.clone(),
                            })
                        }
                        None => Ok(Ballot {
                            option: vote.clone(),
                        }),
                    }
                },
            )?;

            // Find the position of the new vote option and increment it by 1
            let position = poll.options.iter().position(|option| option.0 == vote);
            if position.is_none() {
                return Err(ContractError::Unauthorized {});
            }
            let position = position.unwrap();
            poll.options[position].1 += 1;

            // This stores the updated vote
            POLLS.save(deps.storage, poll_id, &poll)?;
            Ok(Response::new())
        }
        // Poll doesn't exist
        None => Err(ContractError::PollNotFound {}), // Return error
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate}; // Adding execute
    use crate::msg::{ExecuteMsg, InstantiateMsg}; // Adding ExecuteMsg
    // use crate::ContractError;
    use cosmwasm_std::attr; // constructs an attribute
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info}; // mock functions // our instantiate method

    // Fake addresses
    pub const ADDR1: &str = "addr1";
    pub const ADDR2: &str = "addr2";

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        // Mock the contract environment (like a real chain)
        let env = mock_env();
        // Mock the message info, ADDR1 will be the sender, the empty vec means we sent no funds.
        let info = mock_info(ADDR1, &vec![]);

        // Create a message where the sender will be an admin
        let msg = InstantiateMsg { admin: None };
        // Call instantiate, unwrap to assert success
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![attr("action", "instantiate"), attr("admin", ADDR1)]
        )
    }

    #[test]
    fn test_instantiate_with_admin() {
        // Copy paste of test_instantiate but changes None to Some(ADDR2.to_string())
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR2, &vec![]);

        let msg = InstantiateMsg {
            admin: Some(ADDR2.to_string()),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![attr("action", "instantiate"), attr("admin", ADDR2)]
        )
    }

    // Start of new tests

    // Poll valid tests
    #[test]
    fn test_execute_create_poll_valid() {
        // The Mock values like in instantiate
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &vec![]);
        // Instantiate the contract
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // New execute msg
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "Web3Builders".to_string(),
            question: "Will I finish this on time?".to_string(),
            options: vec![
                "Yes".to_string(),
                "No".to_string(),
                "The world will end before that".to_string(),
            ],
        };

        // Unwrap to assert success
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    }

    //Poll invalid tests
    #[test]
    fn test_execute_create_poll_invalid() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &vec![]);
        // Instantiate the contract
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreatePoll {
            poll_id: "001".to_string(),
            question: "How many numbers fit in this poll?".to_string(),
            options: vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
                "6".to_string(),
            ],
        };

        let _err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    }

    // Vote valid
    #[test]
    fn test_execute_vote_valid() {
        // Envorinment
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &vec![]);

        // Instantiate the contract
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Poll created
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "000".to_string(),
            question: "Choose an option".to_string(),
            options: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Vote executed
        let msg = ExecuteMsg::Vote {
            poll_id: "000".to_string(),
            vote: "1".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Update vote
        let msg = ExecuteMsg::Vote {
            poll_id: "000".to_string(),
            vote: "2".to_string(),
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    }

    // Vote invalid section
    #[test]
    fn test_execute_vote_invalid() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &vec![]);
        // Instantiate the contract
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Vote created, poll doesn't exist.
        let msg = ExecuteMsg::Vote {
            poll_id: "000".to_string(),
            vote: "Okonomiyaki".to_string(),
        };
        // Unwrap to assert error
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Poll created
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "000".to_string(),
            question: "Favorite Japanese food".to_string(),
            options: vec![
                "Onigiri".to_string(),
                "Okonomiyaki".to_string(),
                "Ozoni".to_string(),
            ],
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Poll created, Vote done, option 
        let msg = ExecuteMsg::Vote {
            poll_id: "000".to_string(),
            vote: "Pizza".to_string(),
        };
        let _err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    }
}
