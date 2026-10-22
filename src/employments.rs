//! Employment onboarding, defaults and HMRC notices.
use crate::print_json;
use clap::{Args, Subcommand};
use opsd::{OpsdClient, types::*};

#[derive(Debug, Subcommand)]
pub(crate) enum EmploymentsCommand {
    /// List employments and their current tax settings.
    List { business_id: BusinessId },
    /// Read an employment and its retained starter evidence.
    Get {
        business_id: BusinessId,
        employment_id: EmploymentId,
    },
    /// Create a new employment with zero opening YTD. Migrated employments are not yet supported.
    #[command(alias = "new")]
    Create {
        business_id: BusinessId,
        #[arg(long)]
        employee_id: EmployeeId,
        #[arg(long)]
        paye_scheme_id: PayeSchemeId,
        #[arg(long)]
        pay_schedule_id: PayScheduleId,
        /// Contractual employment start date, YYYY-MM-DD.
        #[arg(long)]
        start_date: EmploymentDate,
        #[arg(long)]
        ni_category: NiCategory,
        /// Regular monthly gross pay, in whole pence (no automatic proration).
        #[arg(long)]
        monthly_gross_pay_pence: NonNegativePence,
        #[arg(long)]
        normal_weekly_hours_band: NormalWeeklyHoursBand,
        /// Explicit confirmation: not_applicable. Loan deductions are not supported yet.
        #[arg(long)]
        student_loan: StudentLoan,
        /// Explicit confirmation: not_applicable. Loan deductions are not supported yet.
        #[arg(long)]
        postgraduate_loan: PostgraduateLoan,
        #[command(subcommand)]
        starter: StarterArgs,
    },
    /// Update defaults or starter evidence. Starter changes are restricted after payroll/notices.
    Update {
        business_id: BusinessId,
        employment_id: EmploymentId,
        #[arg(long)]
        start_date: Option<EmploymentDate>,
        #[arg(long)]
        ni_category: Option<NiCategory>,
        #[arg(long)]
        monthly_gross_pay_pence: Option<NonNegativePence>,
        #[arg(long)]
        normal_weekly_hours_band: Option<NormalWeeklyHoursBand>,
        #[command(subcommand)]
        starter: Option<StarterArgs>,
    },
    /// Apply an HMRC notice now. Existing draft previews will use the updated settings.
    TaxCodeNotice {
        business_id: BusinessId,
        employment_id: EmploymentId,
        #[arg(long)]
        tax_year_start: i32,
        /// Unsuffixed tax code; choose the basis with the subcommand.
        #[arg(long)]
        tax_code: String,
        #[command(subcommand)]
        basis: NoticeBasisArgs,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum StarterArgs {
    /// Use the P45 from the last previous job.
    P45 {
        #[arg(long)]
        leaving_date: EmploymentDate,
        #[arg(long)]
        tax_code: String,
        /// Required when the P45 is older than the tax year preceding employment start.
        #[arg(long)]
        starter_declaration: Option<StarterDeclaration>,
        #[command(subcommand)]
        basis: P45BasisArgs,
    },
    /// Record the employee's starter checklist declaration.
    Checklist {
        #[arg(long)]
        declaration: StarterDeclaration,
    },
    /// Record that neither a P45 nor a completed checklist is available.
    NotProvided,
}
#[derive(Debug, Subcommand)]
pub(crate) enum P45BasisArgs {
    Cumulative {
        #[arg(long)]
        total_pay_to_date_pence: NonNegativePence,
        #[arg(long, allow_hyphen_values = true)]
        total_tax_to_date_pence: i64,
    },
    /// W1/M1 P45: no previous pay/tax figures are imported.
    NonCumulative,
}
impl From<StarterArgs> for StarterInformation {
    fn from(value: StarterArgs) -> Self {
        match value {
            StarterArgs::P45 {
                leaving_date,
                tax_code,
                starter_declaration,
                basis,
            } => Self::P45 {
                p45: opsd::types::P45 {
                    leaving_date,
                    tax_code,
                    tax_details: match basis {
                        P45BasisArgs::Cumulative {
                            total_pay_to_date_pence,
                            total_tax_to_date_pence,
                        } => P45TaxDetails::Cumulative {
                            total_pay_to_date_pence,
                            total_tax_to_date_pence,
                        },
                        P45BasisArgs::NonCumulative => P45TaxDetails::NonCumulative {},
                    },
                },
                starter_declaration,
            },
            StarterArgs::Checklist { declaration } => Self::Checklist { declaration },
            StarterArgs::NotProvided => Self::NotProvided {},
        }
    }
}
#[derive(Debug, Args)]
pub(crate) struct PreviousEmploymentArgs {
    /// Earlier-job taxable pay: replaces the stored total, never this employment's YTD.
    #[arg(long, required = false, requires = "previous_tax_paid_pence")]
    previous_taxable_pay_pence: NonNegativePence,
    /// Earlier-job tax, including negative refunds.
    #[arg(
        long,
        required = false,
        requires = "previous_taxable_pay_pence",
        allow_hyphen_values = true
    )]
    previous_tax_paid_pence: i64,
}
#[derive(Debug, Subcommand)]
pub(crate) enum NoticeBasisArgs {
    /// Omit earlier-job totals to retain stored cumulative figures. Switching from W1/M1 requires both totals.
    Cumulative {
        #[command(flatten)]
        previous: Option<PreviousEmploymentArgs>,
    },
    /// Apply W1/M1 treatment, without earlier-job totals.
    NonCumulative,
}
impl NoticeBasisArgs {
    fn into_notice(self, tax_year_start: i32, tax_code: String) -> TaxCodeNotice {
        match self {
            Self::Cumulative { previous } => TaxCodeNotice::Cumulative {
                tax_year_start,
                tax_code,
                previous_employment: previous.map(|p| PreviousEmploymentAmounts {
                    taxable_pay_pence: p.previous_taxable_pay_pence,
                    tax_paid_pence: p.previous_tax_paid_pence,
                }),
            },
            Self::NonCumulative => TaxCodeNotice::NonCumulative {
                tax_year_start,
                tax_code,
            },
        }
    }
}
pub(crate) async fn execute(
    client: &OpsdClient,
    command: EmploymentsCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        EmploymentsCommand::List { business_id } => {
            print_json(&client.list_employments(business_id).await?)?
        }
        EmploymentsCommand::Get {
            business_id,
            employment_id,
        } => print_json(&client.get_employment(business_id, employment_id).await?)?,
        EmploymentsCommand::Create {
            business_id,
            employee_id,
            paye_scheme_id,
            pay_schedule_id,
            start_date,
            ni_category,
            monthly_gross_pay_pence,
            normal_weekly_hours_band,
            student_loan,
            postgraduate_loan,
            starter,
        } => print_json(
            &client
                .create_employment(
                    business_id,
                    &CreateEmploymentRequest {
                        employee_id,
                        paye_scheme_id,
                        pay_schedule_id,
                        start_date,
                        ni_category,
                        monthly_gross_pay_pence,
                        normal_weekly_hours_band,
                        student_loan,
                        postgraduate_loan,
                        starter_information: starter.into(),
                    },
                )
                .await?,
        )?,
        EmploymentsCommand::Update {
            business_id,
            employment_id,
            start_date,
            ni_category,
            monthly_gross_pay_pence,
            normal_weekly_hours_band,
            starter,
        } => {
            if start_date.is_none()
                && ni_category.is_none()
                && monthly_gross_pay_pence.is_none()
                && normal_weekly_hours_band.is_none()
                && starter.is_none()
            {
                return Err(
                    "supply at least one employment setting or a starter subcommand".into(),
                );
            }
            print_json(
                &client
                    .update_employment(
                        business_id,
                        employment_id,
                        &UpdateEmploymentRequest::Settings(UpdateEmploymentSettings {
                            start_date,
                            ni_category,
                            monthly_gross_pay_pence,
                            normal_weekly_hours_band,
                            starter_information: starter.map(Into::into),
                        }),
                    )
                    .await?,
            )?;
        }
        EmploymentsCommand::TaxCodeNotice {
            business_id,
            employment_id,
            tax_year_start,
            tax_code,
            basis,
        } => print_json(
            &client
                .update_employment(
                    business_id,
                    employment_id,
                    &UpdateEmploymentRequest::TaxCodeNotice(ApplyEmploymentTaxCodeNotice {
                        tax_code_notice: basis.into_notice(tax_year_start, tax_code),
                        monthly_gross_pay_pence: None,
                        ni_category: None,
                        normal_weekly_hours_band: None,
                    }),
                )
                .await?,
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: EmploymentsCommand,
    }
    const B: &str = "22222222-2222-4222-8222-222222222222";
    const E: &str = "77777777-7777-4777-8777-777777777777";
    #[test]
    fn cumulative_notice_accepts_no_totals_or_a_complete_pair() {
        for extra in [
            vec![],
            vec![
                "--previous-taxable-pay-pence",
                "0",
                "--previous-tax-paid-pence",
                "-20",
            ],
        ] {
            let mut args = vec![
                "opsctl",
                "tax-code-notice",
                B,
                E,
                "--tax-year-start",
                "2026",
                "--tax-code",
                "1257L",
                "cumulative",
            ];
            args.extend(extra.clone());
            let cli = TestCli::try_parse_from(args).unwrap();
            let EmploymentsCommand::TaxCodeNotice {
                basis,
                tax_year_start,
                tax_code,
                ..
            } = cli.command
            else {
                panic!()
            };
            let TaxCodeNotice::Cumulative {
                previous_employment,
                ..
            } = basis.into_notice(tax_year_start, tax_code)
            else {
                panic!()
            };
            assert_eq!(previous_employment.is_some(), !extra.is_empty());
            if let Some(previous) = previous_employment {
                assert_eq!(previous.tax_paid_pence, -20);
            }
        }
        for extra in [
            vec!["--previous-taxable-pay-pence", "0"],
            vec!["--previous-tax-paid-pence", "0"],
        ] {
            let mut args = vec![
                "opsctl",
                "tax-code-notice",
                B,
                E,
                "--tax-year-start",
                "2026",
                "--tax-code",
                "1257L",
                "cumulative",
            ];
            args.extend(extra);
            assert!(TestCli::try_parse_from(args).is_err());
        }
    }
    #[test]
    fn incompatible_notice_and_p45_arguments_are_rejected() {
        assert!(
            TestCli::try_parse_from([
                "opsctl",
                "tax-code-notice",
                B,
                E,
                "--tax-year-start",
                "2026",
                "--tax-code",
                "BR",
                "non-cumulative",
                "--previous-taxable-pay-pence",
                "0",
                "--previous-tax-paid-pence",
                "0"
            ])
            .is_err()
        );
        let base = [
            "opsctl",
            "create",
            B,
            "--employee-id",
            E,
            "--paye-scheme-id",
            E,
            "--pay-schedule-id",
            E,
            "--start-date",
            "2026-09-01",
            "--ni-category",
            "A",
            "--monthly-gross-pay-pence",
            "300000",
            "--normal-weekly-hours-band",
            "thirty_or_more",
            "--student-loan",
            "not_applicable",
            "--postgraduate-loan",
            "not_applicable",
        ];
        for suffix in [
            vec!["checklist", "--declaration", "A"],
            vec!["not-provided"],
            vec![
                "p45",
                "--leaving-date",
                "2026-08-31",
                "--tax-code",
                "1257L",
                "cumulative",
                "--total-pay-to-date-pence",
                "500000",
                "--total-tax-to-date-pence",
                "-20",
            ],
            vec![
                "p45",
                "--leaving-date",
                "2024-08-31",
                "--tax-code",
                "1257L",
                "--starter-declaration",
                "A",
                "non-cumulative",
            ],
        ] {
            TestCli::try_parse_from(base.into_iter().chain(suffix)).unwrap();
        }
        assert!(TestCli::try_parse_from(base).is_err());
        assert!(
            TestCli::try_parse_from(base.into_iter().chain(["checklist", "--declaration", "D"]))
                .is_err()
        );
        assert!(
            TestCli::try_parse_from(base.into_iter().chain([
                "p45",
                "--leaving-date",
                "2026-08-31",
                "--tax-code",
                "1257L",
                "cumulative",
                "--total-pay-to-date-pence",
                "500000"
            ]))
            .is_err()
        );
    }
}
