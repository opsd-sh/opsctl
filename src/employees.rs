//! Commands for employees belonging to a business.

use clap::{ArgGroup, Subcommand};
use opsd::{
    OpsdClient,
    types::{
        BusinessId, CreateEmployeeRequest, EmployeeForenames, EmployeeId, EmployeeSurname,
        UpdateEmployeeRequest,
    },
};

use crate::print_json;

#[derive(Debug, Subcommand)]
pub(crate) enum EmployeesCommand {
    /// List a business's employees.
    List {
        /// Public ID of the business.
        business_id: BusinessId,
    },
    /// Create an employee.
    Create {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Forenames in order. Quote any individual forename containing spaces.
        #[arg(long, num_args = 1.., required = true)]
        forenames: Vec<String>,
        /// Surname.
        #[arg(long)]
        surname: EmployeeSurname,
    },
    /// Get an employee.
    Get {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the employee.
        employee_id: EmployeeId,
    },
    /// Change an employee's name.
    #[command(group(
        ArgGroup::new("changes")
            .required(true)
            .multiple(true)
            .args(["forenames", "surname"])
    ))]
    Update {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the employee.
        employee_id: EmployeeId,
        /// Replace all forenames, in order. Quote any forename containing spaces.
        #[arg(long, num_args = 1..)]
        forenames: Vec<String>,
        /// Replacement surname.
        #[arg(long)]
        surname: Option<EmployeeSurname>,
    },
}

/// Executes an employee command against the authenticated public API.
pub(crate) async fn execute(
    client: &OpsdClient,
    command: EmployeesCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        EmployeesCommand::List { business_id } => {
            print_json(&client.list_employees(business_id).await?)?
        }
        EmployeesCommand::Create {
            business_id,
            forenames,
            surname,
        } => {
            let request = CreateEmployeeRequest {
                forenames: EmployeeForenames::parse(forenames)?,
                surname,
            };
            print_json(&client.create_employee(business_id, &request).await?)?;
        }
        EmployeesCommand::Get {
            business_id,
            employee_id,
        } => print_json(&client.get_employee(business_id, employee_id).await?)?,
        EmployeesCommand::Update {
            business_id,
            employee_id,
            forenames,
            surname,
        } => {
            let request = UpdateEmployeeRequest {
                forenames: (!forenames.is_empty())
                    .then(|| EmployeeForenames::parse(forenames))
                    .transpose()?,
                surname,
            };
            print_json(
                &client
                    .update_employee(business_id, employee_id, &request)
                    .await?,
            )?;
        }
    }

    Ok(())
}
