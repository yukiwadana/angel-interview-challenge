use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::state::{config, config_read, State};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, UsersResponse, ExistResponse};

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;

pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  let state = State {
    owner: deps.api.addr_validate(&msg.owner)?,
    users: vec![],
  };

  config(deps.storage).save(&state)?;
  Ok(Response::default())
}

pub fn execute(
  deps: &mut DepsMut,
  _env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  match msg {
    ExecuteMsg::AddUser {user} => add_user(deps, info, user),
    ExecuteMsg::RemoveUser {user} => remove_user(deps, info, user),
    ExecuteMsg::UpdateUsers {add, remove} => update_users(deps, info, add, remove)
  }
}

fn add_user(
  deps: &mut DepsMut,
  info: MessageInfo,
  user: String,
) -> Result<Response, ContractError> {
  let mut state = config(deps.storage).load()?;
  if info.sender != state.owner {
    return Err(ContractError::Unauthorized {});
  }

  validate_name(&user)?;
  let user_addr = deps.api.addr_validate(&user)?;

  let index = state.users.iter().position(|x| *x == user_addr).unwrap();
  if index >= 0 {
    Ok(Response::default())
  } else {
    state.users.push(user_addr);
    config(deps.storage).save(&state)?;
    Ok(Response::default())
  }
}

fn remove_user(
  deps: &mut DepsMut,
  info: MessageInfo,
  user: String,
) -> Result<Response, ContractError> {
  let mut state = config(deps.storage).load()?;

  if info.sender != state.owner {
    return Err(ContractError::Unauthorized {});
  }

  let user_addr = deps.api.addr_validate(&user)?;
  let index = state.users.iter().position(|x| *x == user_addr).unwrap();
  if index >= 0 {
    state.users.remove(index);
    config(deps.storage).save(&state)?;
  }

  Ok(Response::default())
}

fn update_users(
  deps: &mut DepsMut,
  info: MessageInfo,
  add: Vec<String>,
  remove: Vec<String>,
) -> Result<Response, ContractError> {
  let state = config(deps.storage).load()?;

  if info.sender.clone() != state.owner {
    return Err(ContractError::Unauthorized {});
  }

  for user in add.into_iter() {
    let _res = add_user(deps, info.clone(), user);
  }

  for user in remove.into_iter() {
    let _res = remove_user(deps, info.clone(), user);
  }

  Ok(Response::default())
}

pub fn query(
  deps: Deps,
  _env: Env,
  msg: QueryMsg,
) -> StdResult<Binary> {
  match msg {
    QueryMsg::GetUsers {} => to_binary(&get_users(deps)?),
    QueryMsg::GetUser {user} => to_binary(&get_user(deps, user)?),
  }
}

fn get_users(
  deps: Deps,
) -> StdResult<UsersResponse> {
  let state = config_read(deps.storage).load()?;
  
  Ok(UsersResponse {users: state.users})
}

fn get_user(
  deps: Deps,
  user: String,
) -> StdResult<ExistResponse> {
  let state = config_read(deps.storage).load()?;
  let user_addr = deps.api.addr_validate(&user)?;
  let exist = state.users.contains(&user_addr);
  
  Ok(ExistResponse {exist})
}

// let's not import a regexp library and just do these checks by hand
fn invalid_char(c: char) -> bool {
  let is_valid =
    (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c == '.' || c == '-' || c == '_');
  !is_valid
}

/// validate_name returns an error if the name is invalid
/// (we require 3-64 lowercase ascii letters, numbers, or . - _)
fn validate_name(name: &str) -> Result<(), ContractError> {
  let length = name.len() as u64;
  if (name.len() as u64) < MIN_NAME_LENGTH {
    Err(ContractError::NameTooShort {
      length,
      min_length: MIN_NAME_LENGTH,
    })
  } else if (name.len() as u64) > MAX_NAME_LENGTH {
    Err(ContractError::NameTooLong {
      length,
      max_length: MAX_NAME_LENGTH,
    })
  } else {
    match name.find(invalid_char) {
      None => Ok(()),
      Some(bytepos_invalid_char_start) => {
        let c = name[bytepos_invalid_char_start..].chars().next().unwrap();
        Err(ContractError::InvalidCharacter { c })
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
  use cosmwasm_std::{coins, Addr};

  fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
      owner: "beneficiary".to_string(),
      users: vec![],
    }
  }

  #[test]
  fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let env = mock_env();
    let info = mock_info("creator", &coins(1000, "earth"));

    let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    
    // it worked, let's query the state
    let state = config_read(&mut deps.storage).load().unwrap();
    assert_eq!(
      state,
      State {
        users: vec![],
        owner: Addr::unchecked("beneficiary"),
      }
    );
  }

  #[test]
  fn execute_add_user() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    let init_res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, init_res.messages.len());

    // AddUser test
    let msg = ExecuteMsg::AddUser { user: "addr000".to_string()};
    let env = mock_env();
    let info = mock_info("beneficiary", &[]);
    let execute_res = execute(&mut deps.as_mut(), env, info, msg.clone());
    match execute_res.unwrap_err() {
      ContractError::Unauthorized { .. } => {}
      e => panic!("Unexpected error: {:?}", e),
    }
  }

  #[test]
  fn execute_remove_user() {
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    let init_res = instantiate(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, init_res.messages.len());

    // RemoveUser test
    let msg = ExecuteMsg::RemoveUser { user: "addr000".to_string()};
    let env = mock_env();
    let info = mock_info("beneficiary", &[]);
    let execute_res = execute(&mut deps.as_mut(), env, info, msg.clone());
    match execute_res.unwrap_err() {
      ContractError::Unauthorized { .. } => {}
      e => panic!("Unexpected error: {:?}", e),
    }
  }

  #[test]
  fn query_get_users() {
    execute_add_user();
    let deps = mock_dependencies(&[]);
    let addr0000 = Addr::unchecked("addr0000");
    // now let's query
    let query_response = get_users(deps.as_ref()).unwrap();
    assert_eq!(query_response.users, vec![addr0000]);
  }

  #[test]
  fn query_get_user() {
    execute_add_user();
    let deps = mock_dependencies(&[]);
    let addr0000 = "addr0000".to_string();
    // now let's query
    let query_response = get_user(deps.as_ref(), addr0000).unwrap();
    assert_eq!(query_response.exist, true);
  }
}
