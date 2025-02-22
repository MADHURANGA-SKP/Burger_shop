#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod burger_shop {

    use ink::prelude::format;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    #[ink(storage)]
    pub struct BurgerShop {
        orders: Vec<(u32, Order)>,
        orders_mapping: Mapping<u32, Order>,
    }

    //the order type
    #[derive(Encode, Decode, Debug,Clone)] //PartialEq 
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Order {
        list_of_items: Vec<FoodItem>,
        customer: AccountId,
        total_price: Balance,
        paid: bool,
        order_id: u32,
    }

    //implement methods for Order struct
    impl Order {
        fn new(list_of_items: Vec<FoodItem>, customer: AccountId, id: u32) -> Self {
            let total_price = Order::total_price(&list_of_items);
            Self {
                list_of_items,
                customer,
                total_price,
                paid: false,
                order_id: id, //default is getting ingreediants in this case
            }
        }

        //call total by itertating thorugh the list_of_items Vec
        #[allow(clippy::arithmetic_side_effects)]
        fn total_price(list_of_items: &Vec<FoodItem>) -> Balance {
            let mut total = 0;
            for item in list_of_items {
                total += item.price()
            }
            total
        }
    }

    //Food item type, basically for each food item
    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct FoodItem {
        burger_menu: BurgerMenu,
        amount: u32,
    }

    //implement methods for FoodItem struct
    impl FoodItem {
        #[allow(clippy::arithmetic_side_effects)]
        fn price(&self) -> Balance {
            match self.burger_menu {
                BurgerMenu::CheeseBurger => BurgerMenu::CheeseBurger.price() * self.amount as u128,
                BurgerMenu::ChickenBurger => {
                    BurgerMenu::ChickenBurger.price() * self.amount as u128
                }
                BurgerMenu::VegiBurger => BurgerMenu::VegiBurger.price() * self.amount as u128,
            }
        }
    }

    //Burger type,
    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    #[allow(clippy::cast_possible_truncation)]
    pub enum BurgerMenu {
        CheeseBurger,
        ChickenBurger,
        VegiBurger,
    }

    //implement methods for BurgerMenu enum
    impl BurgerMenu {
        fn price(&self) -> Balance {
            match self {
                Self::CheeseBurger => 12,
                Self::ChickenBurger => 15,
                Self::VegiBurger => 10,
            }
        }
    }

    /// Event emits when a token transfer occurs
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    /// Event wen shop owner get all oders from storage
    #[ink(event)]
    pub struct GetAllOrders {
        #[ink(topic)]
        orders: Vec<(u32, Order)>,
    }

    /// Event when shop_owner get single order
    #[ink(event)]
    pub struct GetSingleOrder {
        #[ink(topic)]
        single_order: Order,
    }

    /// Event when shop_owner creates his shop
    #[ink(event)]
    pub struct CreatedShopAndStorage {
        #[ink(topic)]
        orders: Vec<(u32, Order)>,
        // this only contains a vector because `Mapping` doesn't implement "encode" trait,
        //this means you can't encode or decode it for operational purposes,
        //it also means you can't return `Mapping` as a result for your contract calls
    }

    // For catching errors that happens during shop operations
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[allow(clippy::cast_possible_truncation)]
    pub enum BurgerShopError {
        //Error type for different errors
        PaymentErrors,
        OrderNotCompleted,
    }

    //define Result Type
    pub type Result<T> = core::result::Result<T, BurgerShopError>;

    //Implment the Contract
    impl BurgerShop {
        #[ink(constructor)]
        //function that instantiates the smart contract with outside world
        pub fn new() -> Self {
            let order_storage_vector: Vec<(u32, Order)> = Vec::new();
            let order_storage_mapping = Mapping::new();

            Self {
                orders: order_storage_vector,
                orders_mapping: order_storage_mapping,
            }
        }

        /// takes the order and makes the payment,
        /// we aren't implementing cart feature here for simplicity purposes,
        #[ink(message, payable)]
        pub fn take_order_and_payment(&mut self, list_of_items: Vec<FoodItem>) -> Result<Order> {
            //get caller account id
            let caller = Self::env().caller();

            //this is assertion is opinionated,
            //if you don't want to limit the shop owner from creating an order,
            //you can remove this line
            assert!(
                caller != Self::env().account_id(),
                "You're not an customer.!"
            );

            // assert the order contains at least 1 item
            for items in &list_of_items {
                assert!(items.amount > 0, "Can't take empty order");
            }

            //our own local id,
            //you can change this to a hash if you want,
            //but remember to make the neccessary type changes too!
            #[allow(clippy::cast_possible_truncation)]
            let id = self.orders.len() as u32;

            //cal and set order price
            let total_price = Order::total_price(&list_of_items);
            let mut order = Order::new(list_of_items, caller, id);
            order.total_price = total_price;

            assert!(
                order.paid == false,
                "Can't pay for an order that is paid for already"
            );

            let multiply: Balance = 1_000_000_000_000; // this equals to 1 Azero, so we doing some conversion
            let transfered_value = self.env().transferred_value();

            // assert the value sent == total price
            assert!(
                transfered_value
                    == order
                        .total_price
                        .checked_mul(multiply)
                        .expect("overflow.!!!"),
                "{}",
                format!(
                    "Please pay complete amount which is {} : ",
                    order.total_price
                )
            );

            ink::env::debug_println!("Expected Value {}", order.total_price);

            ink::env::debug_println!(
                "Expected Recvied payment without conversion {}",
                transfered_value
            );

            //make payment
            match self
                .env()
                .transfer(self.env().account_id(), order.total_price)
            {
                Ok(_) => {
                    // get current length of the list orders in storage,
                    //this will act as our unique id
                     
                    let id = self.orders.len() as u32;

                    //mark order as paid
                    order.paid = true;

                    //emit event
                    self.env().emit_event(Transfer {
                        from: Some(order.customer),
                        to: Some(self.env().account_id()),
                        value: order.total_price,
                    });

                    //push to storage
                    self.orders_mapping.insert(id, &order);
                    self.orders.push((id, order.clone()));
                    Ok(order)
                }
                Err(_) => Err(BurgerShopError::PaymentErrors),
            }
        }

        #[ink(message)]
        //get single order from the burger shop
        pub fn get_single_order(&self, id: u32) -> Order {
            let single_order = self
                .orders_mapping
                .get(id)
                .expect("Oh no, Order Not Found.!");
            single_order
        }

        #[ink(message)]
        pub fn get_orders(&self) -> Option<Vec<(u32, Order)>> {
            //get all orders
            let get_all_orders = &self.orders;

            if get_all_orders.len() > 0 {
                Some(get_all_orders.to_vec()) // converts ref to an owned/new vector
            } else {
                None
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use ink::env::DefaultEnvironment;

    use crate::{
        burger_shop::{BurgerShop, FoodItem},
        *,
    };
    // use crate::burger_shop::BurgerShop;

    #[test]
    fn first_test() {
        assert!(2 == 2);
    }

    // #[ink::test]
    // fn first_integration_test_works() {
    //     let shop = BurgerShop::new();
    //     assert_eq!(None, shop.get_orders());
    // }

    #[ink::test]
    fn order_and_payment_works() {
        let mut shop = BurgerShop::new();
        // test customer acct
        let customer_account =
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        // // set test tokens into acct
        // ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(customer_account.bob, 100);

        let initial_bal =
            ink::env::test::get_account_balance::<DefaultEnvironment>(customer_account.bob)
                .expect("no bal");

        assert!(initial_bal == 1000_u128);

        // set caller which is the customer_account in this case
        ink::env::test::set_callee::<ink::env::DefaultEnvironment>(customer_account.bob);

        // assert caller
        assert_eq!(
            ink::env::test::callee::<DefaultEnvironment>(),
            customer_account.bob
        );

        // make order
        // let food_items = FoodItem {
        //     burger_menu: burger_shop::BurgerMenu::ChickenBurger,
        //     amount: 2,
        // };

        ink::env::test::set_value_transferred::<DefaultEnvironment>(30);
        let bob_after = ink::env::test::get_account_balance::<DefaultEnvironment>(customer_account.bob);
        dbg!(bob_after);

        ink::env::test::set_caller::<DefaultEnvironment>(customer_account.alice);

        let alice_initial = ink::env::test::get_account_balance::<DefaultEnvironment>(customer_account.alice);

        dbg!(alice_initial.expect("err"));
        //    assert!(initial_bal == 970_u128);
        ink::env::test::set_value_transferred::<DefaultEnvironment>(30);
        assert_eq!(
            ink::env::test::callee::<DefaultEnvironment>(),
            customer_account.bob
        );
        let alice_after = ink::env::test::get_account_balance::<DefaultEnvironment>(customer_account.alice);
        dbg!(alice_after.expect("err"));
        

        // shop.take_order_and_payment(vec![food_items]).expect("something went wrong");
    }
}
