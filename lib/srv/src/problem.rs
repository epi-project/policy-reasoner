use problem_details::ProblemDetails;

#[derive(Debug)]
pub struct Problem(pub ProblemDetails);

impl warp::reject::Reject for Problem {}
