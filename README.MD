# Vehicle Management Lambda

This project is a Lambda function that provides a data access layer for interacting with a DynamoDB database. It defines a trait `DataAccess` that specifies the interface for data access operations, and a struct `DBDataAccess` that implements this trait. 
Here, the database design is inspired from [The Dyanmo book](https://www.dynamodbbook.com/) by Alex DeBrie

## Installation

To install the project, clone the repository and run the following command:

```bash
cargo lambda watch
```

## Usage

To use the project, you can create an instance of the `DBDataAccess` struct and call the methods defined in the `DataAccess` trait. Here's an example:

```rust
use vehicle_management_lambda::{DataAccess, DBDataAccess};

#[tokio::main]
async fn main() {
    let table_name = "VehicleDB";
    let sdk_config = aws_config::defaults().load().await;
    let client = aws_sdk_dynamodb::Client::new(&sdk_config);

    let data_access = DBDataAccess::new(client, table_name.to_string());

    // Call the create_user method
    let user = User {
        username: "example_user".to_string(),
        password: "example_password".to_string(),
    };
    data_access.create_user(user).await.unwrap();

    // Call the get_session method
    let user = User {
        username: "example_user".to_string(),
        password: "example_password".to_string(),
    };
    let session = data_access.get_session(user).await.unwrap();

    // ...
}
```

## Testing

To run the tests, use the following command:

```bash
cargo test
```

## Contributing

Contributions are welcome! If you find a bug or have a feature request, please open an issue or submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).
```

Feel free to modify this template to fit your project's specific needs.
