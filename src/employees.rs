//! Commands for employees belonging to a business.

use clap::{ArgGroup, Args, Subcommand};
use opsd::types::*;
use opsd::{
    OpsdClient,
    types::{
        BusinessId, CreateEmployeeRequest, EmployeeForenames, EmployeeId, EmployeeSurname,
        UpdateEmployeeRequest,
    },
};
use std::path::PathBuf;

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
        #[command(flatten)]
        details: NewEmployeeDetails,
    },
    /// Get an employee.
    Get {
        /// Public ID of the business.
        business_id: BusinessId,
        /// Public ID of the employee.
        employee_id: EmployeeId,
    },
    /// Change employee details, subject to payroll history restrictions.
    #[command(group(ArgGroup::new("changes").required(true).multiple(true).args([
        "forenames", "surname", "title", "clear_title", "passport_number", "clear_passport_number",
        "date_of_birth", "sex", "address", "national_insurance_number", "clear_national_insurance_number"
    ])))]
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
        #[command(flatten)]
        details: ChangedEmployeeDetails,
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
            details,
        } => {
            let request = CreateEmployeeRequest {
                forenames: EmployeeForenames::parse(forenames)?,
                surname,
                title: details.title,
                passport_number: details.passport_number,
                date_of_birth: details.date_of_birth,
                sex: details.sex,
                address: serde_json::from_reader(std::fs::File::open(details.address)?)?,
                national_insurance_number: details.national_insurance_number,
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
            details,
        } => {
            let request = UpdateEmployeeRequest {
                forenames: (!forenames.is_empty())
                    .then(|| EmployeeForenames::parse(forenames))
                    .transpose()?,
                surname,
                title: field_update(details.title, details.clear_title),
                passport_number: field_update(
                    details.passport_number,
                    details.clear_passport_number,
                ),
                national_insurance_number: field_update(
                    details.national_insurance_number,
                    details.clear_national_insurance_number,
                ),
                date_of_birth: details.date_of_birth,
                sex: details.sex,
                address: details
                    .address
                    .map(|p| -> Result<_, Box<dyn std::error::Error>> {
                        Ok(serde_json::from_reader(std::fs::File::open(p)?)?)
                    })
                    .transpose()?,
            };
            if serde_json::to_value(&request)?
                .as_object()
                .is_some_and(|o| o.is_empty())
            {
                return Err("supply at least one employee detail to update".into());
            }
            print_json(
                &client
                    .update_employee(business_id, employee_id, &request)
                    .await?,
            )?;
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
pub(crate) struct NewEmployeeDetails {
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    passport_number: Option<String>,
    #[arg(long)]
    date_of_birth: DateOfBirth,
    /// Sex recorded for payroll: M or F.
    #[arg(long)]
    sex: EmployeeSex,
    /// JSON address file: type uk, foreign or lines.
    #[arg(long)]
    address: PathBuf,
    #[arg(long)]
    national_insurance_number: Option<String>,
}
#[derive(Debug, Args)]
pub(crate) struct ChangedEmployeeDetails {
    #[arg(long, conflicts_with = "clear_title")]
    title: Option<String>,
    #[arg(long)]
    clear_title: bool,
    #[arg(long, conflicts_with = "clear_passport_number")]
    passport_number: Option<String>,
    #[arg(long)]
    clear_passport_number: bool,
    #[arg(long)]
    date_of_birth: Option<DateOfBirth>,
    #[arg(long)]
    sex: Option<EmployeeSex>,
    /// JSON address file: type uk, foreign or lines.
    #[arg(long)]
    address: Option<PathBuf>,
    #[arg(long, conflicts_with = "clear_national_insurance_number")]
    national_insurance_number: Option<String>,
    #[arg(long)]
    clear_national_insurance_number: bool,
}
fn field_update(value: Option<String>, clear: bool) -> FieldUpdate<String> {
    if clear {
        FieldUpdate::Clear
    } else {
        value.map(FieldUpdate::Set).unwrap_or_default()
    }
}
