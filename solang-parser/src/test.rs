// SPDX-License-Identifier: Apache-2.0

use crate::lexer::Lexer;
use crate::pt::*;
use crate::solidity;
use pretty_assertions::assert_eq;
use std::sync::mpsc;
use std::time::Duration;
use std::{fs, path::Path, thread};
use walkdir::WalkDir;

#[test]
fn print_test() {
    let src = r#"
// SPDX-License-Identifier: MIT

pragma solidity ^0.8.7;

import "@chainlink/contracts/src/v0.8/interfaces/VRFCoordinatorV2Interface.sol";
import "@chainlink/contracts/src/v0.8/VRFConsumerBaseV2.sol";
import "@chainlink/contracts/src/v0.8/interfaces/KeeperCompatibleInterface.sol";

error CharityRaffle__NotJackpotValue();
error CharityRaffle__FundingContractFailed();
error CharityRaffle__UpkeepNotNeeded(
    uint256 currentBalance,
    uint256 numPlayers,
    uint256 raffleState
);
error CharityRaffle__CharityTransferFailed(address charity);
error CharityRaffle__SendMoreToEnterRaffle();
error CharityRaffle__RaffleNotOpen();
error CharityRaffle__NotValidCharityChoice();
error CharityRaffle__JackpotTransferFailed();
error CharityRaffle__MustBeFunder();
error CharityRaffle__NoCharityWinner();
error CharityRaffle__RaffleNotClosed();
error CharityRaffle__MatchAlreadyFunded();
error CharityRaffle__IncorrectMatchValue();
error CharityRaffle__FundingToMatchTransferFailed();
error CharityRaffle__ContractNotFunded();
error CharityRaffle__DonationMatchFailed();

/**@title A sample Charity Raffle Contract originally @author Patrick Collins
 * @notice This contract creates a lottery in which players enter by donating to 1 of 3 charities
 * @dev This implements the Chainlink VRF Version 2
 */

contract CharityRaffle is VRFConsumerBaseV2, KeeperCompatibleInterface {
    /* Type declarations */
    enum RaffleState {
        OPEN,
        CALCULATING,
        CLOSED
    }
    enum CharityChoice {
        CHARITY1,
        CHARITY2,
        CHARITY3
    }
    /* State variables */
    // Chainlink VRF Variables
    VRFCoordinatorV2Interface private immutable i_vrfCoordinator;
    uint16 private constant REQUEST_CONFIRMATIONS = 3;
    uint32 private constant NUM_WORDS = 4;
    uint32 private immutable i_callbackGasLimit;
    uint64 private immutable i_subscriptionId;
    bytes32 private immutable i_gasLane;

    // Lottery Variables
    uint256 private immutable i_duration;
    uint256 private s_startTime;
    uint256 private immutable i_entranceFee;
    uint256 private immutable i_jackpot;
    uint256 private s_highestDonations;
    address private s_recentWinner;
    address private immutable i_charity1;
    address private immutable i_charity2;
    address private immutable i_charity3;
    address private immutable i_fundingWallet;
    address private s_charityWinner;
    bool private s_matchFunded;

    address[] private s_players;
    RaffleState private s_raffleState;

    mapping(address => uint256) donations;

    /* Events */
    event RequestedRaffleWinner(uint256 indexed requestId);
    event RaffleEnter(address indexed player);
    event WinnerPicked(address indexed player);
    event CharityWinnerPicked(address indexed charity);

    /* Modifiers */
    modifier onlyFunder() {
        if (msg.sender != i_fundingWallet) {
            revert CharityRaffle__MustBeFunder();
        }
        _;
    }

    modifier charityWinnerPicked() {
        if (s_charityWinner == address(0)) {
            revert CharityRaffle__NoCharityWinner();
        }
        _;
    }

    /* Functions */
    constructor(
        address vrfCoordinatorV2,
        uint64 subscriptionId,
        bytes32 gasLane, // keyHash
        uint256 entranceFee,
        uint256 jackpot,
        uint256 duration,
        uint32 callbackGasLimit,
        address charity1,
        address charity2,
        address charity3,
        address fundingWallet
    ) payable VRFConsumerBaseV2(vrfCoordinatorV2) {
        i_vrfCoordinator = VRFCoordinatorV2Interface(vrfCoordinatorV2);
        i_duration = duration;
        i_gasLane = gasLane;
        i_subscriptionId = subscriptionId;
        i_entranceFee = entranceFee;
        i_jackpot = jackpot;
        s_raffleState = RaffleState.OPEN;
        s_startTime = block.timestamp;
        i_callbackGasLimit = callbackGasLimit;
        i_charity1 = charity1;
        i_charity2 = charity2;
        i_charity3 = charity3;
        i_fundingWallet = fundingWallet;
        (bool success, ) = payable(address(this)).call{value: jackpot}("");
        if (!success) {
            revert CharityRaffle__FundingContractFailed();
        }
    }

    /*
     * @dev function to enter raffle
     * @param charityChoice - should be 0,1,2 to represent CharityChoice enum
     */

    function enterRaffle(CharityChoice charityChoice) external payable {
        if (msg.value < i_entranceFee) {
            revert CharityRaffle__SendMoreToEnterRaffle();
        }
        if (s_raffleState != RaffleState.OPEN) {
            revert CharityRaffle__RaffleNotOpen();
        }
        if (charityChoice == CharityChoice.CHARITY1) {
            (bool success, ) = i_charity1.call{value: msg.value}("");
            if (!success) {
                revert CharityRaffle__CharityTransferFailed(i_charity1);
            }
            donations[i_charity1]++;
        }
        if (charityChoice == CharityChoice.CHARITY2) {
            (bool success, ) = i_charity2.call{value: msg.value}("");
            if (!success) {
                revert CharityRaffle__CharityTransferFailed(i_charity2);
            }
            donations[i_charity2]++;
        }
        if (charityChoice == CharityChoice.CHARITY3) {
            (bool success, ) = i_charity3.call{value: msg.value}("");
            if (!success) {
                revert CharityRaffle__CharityTransferFailed(i_charity3);
            }
            donations[i_charity3]++;
        }
        s_players.push(payable(msg.sender));
        emit RaffleEnter(msg.sender);
    }

    /*
     * This is the function that the Chainlink Keeper nodes call
     * they look for `upkeepNeeded` to return True.
     * the following should be true for this to return true:
     * 1. The lottery is open.
     * 2. Lottery duration time has elapsed
     * 3. The contract has ETH.
     * 4. Implicity, your subscription is funded with LINK.
     */
    function checkUpkeep(
        bytes memory /* checkData */
    )
        public
        view
        override
        returns (
            bool upkeepNeeded,
            bytes memory /* performData */
        )
    {
        bool isOpen = RaffleState.OPEN == s_raffleState;
        bool timeOver = (block.timestamp - s_startTime) >= i_duration;
        bool hasPlayers = s_players.length > 0;
        bool hasBalance = address(this).balance > 0;
        upkeepNeeded = (isOpen && timeOver && hasBalance && hasPlayers);
    }

    function performUpkeep(
        bytes calldata /* performData */
    ) external override {
        (bool upkeepNeeded, ) = checkUpkeep("");
        if (!upkeepNeeded) {
            revert CharityRaffle__UpkeepNotNeeded(
                address(this).balance,
                s_players.length,
                uint256(s_raffleState)
            );
        }
        s_raffleState = RaffleState.CALCULATING;
        uint256 requestId = i_vrfCoordinator.requestRandomWords(
            i_gasLane,
            i_subscriptionId,
            REQUEST_CONFIRMATIONS,
            i_callbackGasLimit,
            NUM_WORDS
        );
        emit RequestedRaffleWinner(requestId);
    }

    /*
     * @dev function to handle raffle winner
     * picks player winner
     * closes raffle
     * checks if there is a tie in highest charity donations
     * if tie, handleTie is called, else, winner declared
     */
    function fulfillRandomWords(
        uint256,
        /* requestId */
        uint256[] memory randomWords
    ) internal override {
        // declare player winner
        uint256 indexOfWinner = randomWords[0] % s_players.length;
        address recentWinner = s_players[indexOfWinner];
        s_recentWinner = recentWinner;
        s_players = new address[](0);
        s_raffleState = RaffleState.CLOSED;
        // handle if there is charity donations tie
        bool tie = checkForTie();
        uint256 charity1Total = donations[i_charity1];
        donations[i_charity1] = 0;
        uint256 charity2Total = donations[i_charity2];
        donations[i_charity2] = 0;
        uint256 charity3Total = donations[i_charity3];
        donations[i_charity3] = 0;
        if (tie) {
            handleTie(randomWords, charity1Total, charity2Total, charity3Total);
        }
        // not a tie
        if (charity1Total > charity2Total && charity1Total > charity3Total) {
            // charity1 wins
            s_highestDonations = charity1Total;
            s_charityWinner = i_charity1;
        }
        if (charity2Total > charity1Total && charity2Total > charity3Total) {
            // charity2 wins
            s_highestDonations = charity2Total;
            s_charityWinner = i_charity2;
        }
        if (charity3Total > charity1Total && charity3Total > charity2Total) {
            // charity3 wins
            s_highestDonations = charity3Total;
            s_charityWinner = i_charity3;
        }
        (bool success, ) = payable(recentWinner).call{value: address(this).balance}(""); // should be i_jackpot
        if (!success) {
            revert CharityRaffle__JackpotTransferFailed();
        }
        emit WinnerPicked(recentWinner);
        if (s_charityWinner != address(0)) {
            emit CharityWinnerPicked(s_charityWinner);
        }
    }

    function checkForTie() internal view returns (bool) {
        return (donations[i_charity1] == donations[i_charity2] ||
            donations[i_charity1] == donations[i_charity3] ||
            donations[i_charity2] == donations[i_charity3]);
    }

    /*
     * @dev function to use Chainlink VRF to break tie and declare Charity Winner
     * optional - instead of requesting 4 random words from Chainlink VRF,
     * could get "sudo" random numbers by taking the hash and abi.encode of one random number
     */
    function handleTie(
        uint256[] memory randomWords,
        uint256 charity1Total,
        uint256 charity2Total,
        uint256 charity3Total
    ) internal {
        // find top two winners
        uint256[] memory data = new uint256[](3);
        data[0] = charity1Total;
        data[1] = charity2Total;
        data[2] = charity3Total;
        uint256[] memory sortedData = sort(data); // sortedData[2] = highest value
        s_highestDonations = sortedData[2];
        // three-way-tie
        if (charity1Total == charity2Total && charity1Total == charity3Total) {
            charity1Total += randomWords[1];
            charity2Total += randomWords[2];
            charity3Total += randomWords[3];
            uint256[] memory newData = new uint256[](3);
            newData[0] = charity1Total;
            newData[1] = charity2Total;
            newData[2] = charity3Total;
            uint256[] memory newSortedData = sort(newData);
            if (newSortedData[2] == charity1Total) {
                // charity1 wins
                s_charityWinner = i_charity1;
            } else if (newSortedData[2] == charity2Total) {
                //charity2 wins
                s_charityWinner = i_charity2;
            } else {
                // charity3 wins
                s_charityWinner = i_charity3;
            }
        }
        // charity1 and charity2 tie
        if (sortedData[2] == charity1Total && sortedData[2] == charity2Total) {
            charity1Total += randomWords[1];
            charity2Total += randomWords[2];
            if (charity1Total > charity2Total) {
                // charity1 wins
                s_charityWinner = i_charity1;
            } else {
                //charity2 wins
                s_charityWinner = i_charity2;
            }
        }
        // charity1 and charity3 tie
        if (sortedData[2] == charity1Total && sortedData[2] == charity3Total) {
            charity1Total += randomWords[1];
            charity3Total += randomWords[2];
            if (charity1Total > charity3Total) {
                // charity1 wins
                s_charityWinner = i_charity1;
            } else {
                //charity3 wins
                s_charityWinner = i_charity3;
            }
        }
        // charity2 and charity3 tie
        if (sortedData[2] == charity2Total && sortedData[2] == charity3Total) {
            charity2Total += randomWords[1];
            charity3Total += randomWords[2];
            if (charity2Total > charity3Total) {
                // charity2 wins
                s_charityWinner = i_charity2;
            } else {
                //charity3 wins
                s_charityWinner = i_charity3;
            }
        }
        if (s_charityWinner != address(0)) {
            emit CharityWinnerPicked(s_charityWinner);
        }
    }

    /*
     * @dev Internal function to find highest scores
     */
    function sort(uint256[] memory data) internal returns (uint256[] memory) {
        quickSort(data, int256(0), int256(data.length - 1));
        return data;
    }

    function quickSort(
        uint256[] memory arr,
        int256 left,
        int256 right
    ) internal {
        int256 i = left;
        int256 j = right;
        if (i == j) return;
        uint256 pivot = arr[uint256(left + (right - left) / 2)];
        while (i <= j) {
            while (arr[uint256(i)] < pivot) i++;
            while (pivot < arr[uint256(j)]) j--;
            if (i <= j) {
                (arr[uint256(i)], arr[uint256(j)]) = (arr[uint256(j)], arr[uint256(i)]);
                i++;
                j--;
            }
        }
        if (left < j) quickSort(arr, left, j);
        if (i < right) quickSort(arr, i, right);
    }

    /*
     * @dev Funding wallet has option to match donations of winning charity
     * fundDonationMatch must be called before donationMatch to transfer funds into contract
     */
    function fundDonationMatch() external payable onlyFunder charityWinnerPicked {
        if (s_raffleState != RaffleState.CLOSED) {
            revert CharityRaffle__RaffleNotClosed();
        }
        if (s_matchFunded) {
            revert CharityRaffle__MatchAlreadyFunded();
        }
        uint256 mostDonations = s_highestDonations;
        if (msg.value < mostDonations * i_entranceFee) {
            revert CharityRaffle__IncorrectMatchValue();
        }
        s_highestDonations = 0;
        s_matchFunded = true;
    }

    /*
     * @dev function to transfer donation match from contract to winner
     */
    function donationMatch() external onlyFunder charityWinnerPicked {
        if (s_raffleState != RaffleState.CLOSED) {
            revert CharityRaffle__RaffleNotClosed();
        }
        if (!s_matchFunded) {
            revert CharityRaffle__FundingToMatchTransferFailed();
        }
        if (address(this).balance <= 0) {
            revert CharityRaffle__ContractNotFunded();
        }
        address charityWinner = s_charityWinner;
        s_charityWinner = address(0);
        s_recentWinner = address(0);
        s_matchFunded = false;
        (bool donationMatched, ) = payable(charityWinner).call{value: address(this).balance}("");
        if (!donationMatched) {
            revert CharityRaffle__DonationMatchFailed();
        }
    }

    /** Getter Functions */

    function getRaffleState() external view returns (RaffleState) {
        return s_raffleState;
    }

    function getNumWords() external pure returns (uint256) {
        return NUM_WORDS;
    }

    function getRequestConfirmations() external pure returns (uint256) {
        return REQUEST_CONFIRMATIONS;
    }

    function getRecentWinner() external view returns (address) {
        return s_recentWinner;
    }

    function getCharityWinner() external view returns (address) {
        return s_charityWinner;
    }

    function getPlayer(uint256 index) external view returns (address) {
        return s_players[index];
    }

    function getAllPlayers() external view returns (address[] memory) {
        return s_players;
    }

    function getCharities() external view returns (address[] memory) {
        address[] memory charities = new address[](3);
        charities[0] = i_charity1;
        charities[1] = i_charity2;
        charities[2] = i_charity3;
        return charities;
    }

    function getDonations(address charity) external view returns (uint256) {
        return donations[charity];
    }

    function getEntranceFee() external view returns (uint256) {
        return i_entranceFee;
    }

    function getNumberOfPlayers() external view returns (uint256) {
        return s_players.length;
    }

    function getFundingWallet() external view returns (address) {
        return i_fundingWallet;
    }

    function getHighestDonations() external view returns (uint256) {
        return s_highestDonations;
    }

    function getJackpot() external view returns (uint256) {
        return i_jackpot;
    }

    function getStartTime() external view returns (uint256) {
        return s_startTime;
    }

    function getDuration() external view returns (uint256) {
        return i_duration;
    }
}

contract Contract {

    function fulfillRandomWords() internal override {
        (bool success, ) = recentWinner.call{value: address(this).balance}("");
    }
}"#;
    let mut comments = Vec::new();
    let lex = Lexer::new(src, 0, &mut comments);
    let pt = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();
    let src_pretty = pt.to_string();
    let lex_pretty = Lexer::new(&src_pretty, 0, &mut comments);
    assert!(solidity::SourceUnitParser::new()
        .parse(src, 0, lex_pretty)
        .is_ok())
    // println!("{:#?}", pt);
    // println!("{}", pt);
}

#[test]
fn parse_test() {
    let src = r#"/// @title Foo
                /// @description Foo
                /// Bar
                contract foo {
                    /**
                    @title Jurisdiction
                    */
                    /// @author Anon
                    /**
                    @description Data for
                    jurisdiction
                    @dev It's a struct
                    */
                    struct Jurisdiction {
                        bool exists;
                        uint keyIdx;
                        bytes2 country;
                        bytes32 region;
                    }
                    string __abba_$;
                    int64 $thing_102;
                }

                function bar() {
                    try sum(1, 1) returns (uint sum) {
                        assert(sum == 2);
                    } catch (bytes memory b) {
                        revert(unicode'très');
                    } catch Error(string memory error) {
                        revert(error);
                    } catch Panic(uint x) {
                        revert('feh');
                    }
                }"#;

    let mut comments = Vec::new();
    let lex = Lexer::new(src, 0, &mut comments);

    let actual_parse_tree = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            loc: Loc::File(0, 92, 702),
            ty: ContractTy::Contract(Loc::File(0, 92, 100)),
            name: Identifier {
                loc: Loc::File(0, 101, 104),
                name: "foo".to_string(),
            },
            base: Vec::new(),
            parts: vec![
                ContractPart::StructDefinition(Box::new(StructDefinition {
                    name: Identifier {
                        loc: Loc::File(0, 419, 431),
                        name: "Jurisdiction".to_string(),
                    },
                    loc: Loc::File(0, 412, 609),
                    fields: vec![
                        VariableDeclaration {
                            loc: Loc::File(0, 458, 469),
                            ty: Expression::Type(Loc::File(0, 458, 462), Type::Bool),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 463, 469),
                                name: "exists".to_string(),
                            },
                        },
                        VariableDeclaration {
                            loc: Loc::File(0, 495, 506),
                            ty: Expression::Type(Loc::File(0, 495, 499), Type::Uint(256)),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 500, 506),
                                name: "keyIdx".to_string(),
                            },
                        },
                        VariableDeclaration {
                            loc: Loc::File(0, 532, 546),
                            ty: Expression::Type(Loc::File(0, 532, 538), Type::Bytes(2)),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 539, 546),
                                name: "country".to_string(),
                            },
                        },
                        VariableDeclaration {
                            loc: Loc::File(0, 572, 586),
                            ty: Expression::Type(Loc::File(0, 572, 579), Type::Bytes(32)),
                            storage: None,
                            name: Identifier {
                                loc: Loc::File(0, 580, 586),
                                name: "region".to_string(),
                            },
                        },
                    ],
                })),
                ContractPart::VariableDefinition(Box::new(VariableDefinition {
                    ty: Expression::Type(Loc::File(0, 630, 636), Type::String),
                    attrs: vec![],
                    name: Identifier {
                        loc: Loc::File(0, 637, 645),
                        name: "__abba_$".to_string(),
                    },
                    loc: Loc::File(0, 630, 645),
                    initializer: None,
                })),
                ContractPart::VariableDefinition(Box::new(VariableDefinition {
                    ty: Expression::Type(Loc::File(0, 667, 672), Type::Int(64)),
                    attrs: vec![],
                    name: Identifier {
                        loc: Loc::File(0, 673, 683),
                        name: "$thing_102".to_string(),
                    },
                    loc: Loc::File(0, 667, 683),
                    initializer: None,
                })),
            ],
        })),
        SourceUnitPart::FunctionDefinition(Box::new(FunctionDefinition {
            loc: Loc::File(0, 720, 735),
            ty: FunctionTy::Function,
            name: Some(Identifier {
                loc: Loc::File(0, 729, 732),
                name: "bar".to_string(),
            }),
            name_loc: Loc::File(0, 729, 732),
            params: vec![],
            attributes: vec![],
            return_not_returns: None,
            returns: vec![],
            body: Some(Statement::Block {
                loc: Loc::File(0, 735, 1147),
                unchecked: false,
                statements: vec![Statement::Try(
                    Loc::File(0, 757, 1129),
                    Expression::FunctionCall(
                        Loc::File(0, 761, 770),
                        Box::new(Expression::Variable(Identifier {
                            loc: Loc::File(0, 761, 764),
                            name: "sum".to_string(),
                        })),
                        vec![
                            Expression::NumberLiteral(
                                Loc::File(0, 765, 766),
                                "1".to_string(),
                                "".to_string(),
                            ),
                            Expression::NumberLiteral(
                                Loc::File(0, 768, 769),
                                "1".to_string(),
                                "".to_string(),
                            ),
                        ],
                    ),
                    Some((
                        vec![(
                            Loc::File(0, 780, 788),
                            Some(Parameter {
                                loc: Loc::File(0, 780, 788),
                                ty: Expression::Type(Loc::File(0, 780, 784), Type::Uint(256)),
                                storage: None,
                                name: Some(Identifier {
                                    loc: Loc::File(0, 785, 788),
                                    name: "sum".to_string(),
                                }),
                            }),
                        )],
                        Box::new(Statement::Block {
                            loc: Loc::File(0, 790, 855),
                            unchecked: false,
                            statements: vec![Statement::Expression(
                                Loc::File(0, 816, 832),
                                Expression::FunctionCall(
                                    Loc::File(0, 816, 832),
                                    Box::new(Expression::Variable(Identifier {
                                        loc: Loc::File(0, 816, 822),
                                        name: "assert".to_string(),
                                    })),
                                    vec![Expression::Equal(
                                        Loc::File(0, 823, 831),
                                        Box::new(Expression::Variable(Identifier {
                                            loc: Loc::File(0, 823, 826),
                                            name: "sum".to_string(),
                                        })),
                                        Box::new(Expression::NumberLiteral(
                                            Loc::File(0, 830, 831),
                                            "2".to_string(),
                                            "".to_string(),
                                        )),
                                    )],
                                ),
                            )],
                        }),
                    )),
                    vec![
                        CatchClause::Simple(
                            Loc::File(0, 856, 950),
                            Some(Parameter {
                                loc: Loc::File(0, 863, 877),
                                ty: Expression::Type(Loc::File(0, 863, 868), Type::DynamicBytes),
                                storage: Some(StorageLocation::Memory(Loc::File(0, 869, 875))),
                                name: Some(Identifier {
                                    loc: Loc::File(0, 876, 877),
                                    name: "b".to_string(),
                                }),
                            }),
                            Statement::Block {
                                loc: Loc::File(0, 879, 950),
                                unchecked: false,
                                statements: vec![Statement::Revert(
                                    Loc::File(0, 905, 927),
                                    None,
                                    vec![Expression::StringLiteral(vec![StringLiteral {
                                        loc: Loc::File(0, 912, 926),
                                        unicode: true,
                                        string: "très".to_string(),
                                    }])],
                                )],
                            },
                        ),
                        CatchClause::Named(
                            Loc::File(0, 951, 1046),
                            Identifier {
                                loc: Loc::File(0, 957, 962),
                                name: "Error".to_string(),
                            },
                            Parameter {
                                loc: Loc::File(0, 963, 982),
                                ty: Expression::Type(Loc::File(0, 963, 969), Type::String),
                                storage: Some(StorageLocation::Memory(Loc::File(0, 970, 976))),
                                name: Some(Identifier {
                                    loc: Loc::File(0, 977, 982),
                                    name: "error".to_string(),
                                }),
                            },
                            Statement::Block {
                                loc: Loc::File(0, 984, 1046),
                                unchecked: false,
                                statements: vec![Statement::Revert(
                                    Loc::File(0, 1010, 1023),
                                    None,
                                    vec![Expression::Variable(Identifier {
                                        loc: Loc::File(0, 1017, 1022),
                                        name: "error".to_string(),
                                    })],
                                )],
                            },
                        ),
                        CatchClause::Named(
                            Loc::File(0, 1047, 1129),
                            Identifier {
                                loc: Loc::File(0, 1053, 1058),
                                name: "Panic".to_string(),
                            },
                            Parameter {
                                loc: Loc::File(0, 1059, 1065),
                                ty: Expression::Type(Loc::File(0, 1059, 1063), Type::Uint(256)),
                                storage: None,
                                name: Some(Identifier {
                                    loc: Loc::File(0, 1064, 1065),
                                    name: "x".to_string(),
                                }),
                            },
                            Statement::Block {
                                loc: Loc::File(0, 1067, 1129),
                                unchecked: false,
                                statements: vec![Statement::Revert(
                                    Loc::File(0, 1093, 1106),
                                    None,
                                    vec![Expression::StringLiteral(vec![StringLiteral {
                                        loc: Loc::File(0, 1100, 1105),
                                        unicode: false,
                                        string: "feh".to_string(),
                                    }])],
                                )],
                            },
                        ),
                    ],
                )],
            }),
        })),
    ]);

    assert_eq!(actual_parse_tree, expected_parse_tree);

    assert_eq!(
        comments,
        vec![
            Comment::DocLine(
                Loc::File(
                    0,
                    0,
                    14,
                ),
                "/// @title Foo".to_string(),
            ),
            Comment::DocLine(
                Loc::File(
                    0,
                    31,
                    51,
                ),
                "/// @description Foo".to_string(),
            ),
            Comment::DocLine(
                Loc::File(
                    0,
                    68,
                    75,
                ),
                "/// Bar".to_string(),
            ),
            Comment::DocBlock(
                Loc::File(
                    0,
                    127,
                    193,
                ),
                "/**\n                    @title Jurisdiction\n                    */".to_string(),
            ),
            Comment::DocLine(
                Loc::File(
                    0,
                    214,
                    230,
                ),
                "/// @author Anon".to_string(),
            ),
            Comment::DocBlock(
                Loc::File(
                    0,
                    251,
                    391,
                ),
                "/**\n                    @description Data for\n                    jurisdiction\n                    @dev It's a struct\n                    */".to_string(),
            ),
        ]
    );
}

#[test]
fn parse_error_test() {
    let src = r#"

        error Outer(uint256 available, uint256 required);

        contract TestToken {
            error NotPending();
            /// Insufficient balance for transfer. Needed `required` but only
            /// `available` available.
            /// @param available balance available.
            /// @param required requested amount to transfer.
            error InsufficientBalance(uint256 available, uint256 required);
        }
        "#;

    let (actual_parse_tree, comments) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 2);

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::ErrorDefinition(Box::new(ErrorDefinition {
            loc: Loc::File(0, 10, 58),
            name: Identifier {
                loc: Loc::File(0, 16, 21),
                name: "Outer".to_string(),
            },
            fields: vec![
                ErrorParameter {
                    ty: Expression::Type(Loc::File(0, 22, 29), Type::Uint(256)),
                    loc: Loc::File(0, 22, 39),
                    name: Some(Identifier {
                        loc: Loc::File(0, 30, 39),
                        name: "available".to_string(),
                    }),
                },
                ErrorParameter {
                    ty: Expression::Type(Loc::File(0, 41, 48), Type::Uint(256)),
                    loc: Loc::File(0, 41, 57),
                    name: Some(Identifier {
                        loc: Loc::File(0, 49, 57),
                        name: "required".to_string(),
                    }),
                },
            ],
        })),
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            loc: Loc::File(0, 69, 438),
            ty: ContractTy::Contract(Loc::File(0, 69, 77)),
            name: Identifier {
                loc: Loc::File(0, 78, 87),
                name: "TestToken".to_string(),
            },
            base: vec![],
            parts: vec![
                ContractPart::ErrorDefinition(Box::new(ErrorDefinition {
                    loc: Loc::File(0, 102, 120),
                    name: Identifier {
                        loc: Loc::File(0, 108, 118),
                        name: "NotPending".to_string(),
                    },
                    fields: vec![],
                })),
                ContractPart::ErrorDefinition(Box::new(ErrorDefinition {
                    loc: Loc::File(0, 365, 427),
                    name: Identifier {
                        loc: Loc::File(0, 371, 390),
                        name: "InsufficientBalance".to_string(),
                    },
                    fields: vec![
                        ErrorParameter {
                            ty: Expression::Type(Loc::File(0, 391, 398), Type::Uint(256)),
                            loc: Loc::File(0, 391, 408),
                            name: Some(Identifier {
                                loc: Loc::File(0, 399, 408),
                                name: "available".to_string(),
                            }),
                        },
                        ErrorParameter {
                            ty: Expression::Type(Loc::File(0, 410, 417), Type::Uint(256)),
                            loc: Loc::File(0, 410, 426),
                            name: Some(Identifier {
                                loc: Loc::File(0, 418, 426),
                                name: "required".to_string(),
                            }),
                        },
                    ],
                })),
            ],
        })),
    ]);

    assert_eq!(actual_parse_tree, expected_parse_tree);

    assert_eq!(
        comments,
        vec![
            Comment::DocLine(
                Loc::File(0, 134, 199,),
                "/// Insufficient balance for transfer. Needed `required` but only".to_owned(),
            ),
            Comment::DocLine(
                Loc::File(0, 212, 238,),
                "/// `available` available.".to_owned(),
            ),
            Comment::DocLine(
                Loc::File(0, 251, 290,),
                "/// @param available balance available.".to_owned(),
            ),
            Comment::DocLine(
                Loc::File(0, 303, 352,),
                "/// @param required requested amount to transfer.".to_owned(),
            )
        ]
    );
}

#[test]
fn test_assembly_parser() {
    let src = r#"
                function bar() {
                    assembly "evmasm" {
                        let x := 0
                        for { let i := 0 } lt(i, 0x100) { i := add(i, 0x20) } {
                            x := /* meh */ add(x, mload(i))

                            if gt(i, 0x10) {
                                break
                            }
                        }

                        let h : u32, y, z : u16 := funcCall()

                        switch x
                        case 0 {
                            revert(0, 0)
                            // feh
                        }
                        default {
                            leave
                        }
                    }

                    assembly {

                        function power(base : u256, exponent) -> result
                        {
                            let y := and("abc":u32, add(3:u256, 2:u256))
                            let result
                        }
                    }
                }"#;

    let mut comments = Vec::new();
    let lex = Lexer::new(src, 0, &mut comments);
    let actual_parse_tree = solidity::SourceUnitParser::new()
        .parse(src, 0, lex)
        .unwrap();

    let expected_parse_tree = SourceUnit(vec![SourceUnitPart::FunctionDefinition(Box::new(
        FunctionDefinition {
            loc: Loc::File(0, 17, 32),
            ty: FunctionTy::Function,
            name: Some(Identifier {
                loc: Loc::File(0, 26, 29),
                name: "bar".to_string(),
            }),
            name_loc: Loc::File(0, 26, 29),
            params: vec![],
            attributes: vec![],
            return_not_returns: None,
            returns: vec![],
            body: Some(Statement::Block {
                loc: Loc::File(0, 32, 1045),
                unchecked: false,
                statements: vec![
                    Statement::Assembly {
                        loc: Loc::File(0, 54, 736),
                        block: YulBlock {
                            loc: Loc::File(0, 72, 736),
                            statements: vec![
                                YulStatement::VariableDeclaration(
                                    Loc::File(0, 98, 108),
                                    vec![YulTypedIdentifier {
                                        loc: Loc::File(0, 102, 103),
                                        id: Identifier {
                                            loc: Loc::File(0, 102, 103),
                                            name: "x".to_string(),
                                        },
                                        ty: None,
                                    }],
                                    Some(YulExpression::NumberLiteral(
                                        Loc::File(0, 107, 108),
                                        "0".to_string(), "".to_string(),
                                        None,
                                    )),
                                ),
                                YulStatement::For(YulFor {
                                    loc: Loc::File(0, 133, 388),
                                    init_block: YulBlock {
                                        loc: Loc::File(0, 137, 151),
                                        statements: vec![YulStatement::VariableDeclaration(
                                            Loc::File(0, 139, 149),
                                            vec![YulTypedIdentifier {
                                                loc: Loc::File(0, 143, 144),
                                                id: Identifier {
                                                    loc: Loc::File(0, 143, 144),
                                                    name: "i".to_string(),
                                                },
                                                ty: None,
                                            }],
                                            Some(YulExpression::NumberLiteral(
                                                Loc::File(0, 148, 149),
                                                "0".to_string(), "".to_string(),
                                                None,
                                            )),
                                        )],
                                    },
                                    condition: YulExpression::FunctionCall(Box::new(YulFunctionCall {
                                        loc: Loc::File(0, 152, 164),
                                        id: Identifier {
                                            loc: Loc::File(0, 152, 154),
                                            name: "lt".to_string(),
                                        },
                                        arguments: vec![
                                            YulExpression::Variable(Identifier {
                                                loc: Loc::File(0, 155, 156),
                                                name: "i".to_string(),
                                            }),
                                            YulExpression::HexNumberLiteral(
                                                Loc::File(0, 158, 163),
                                                "0x100".to_string(),
                                                None,
                                            ),
                                        ],
                                    })),
                                    post_block: YulBlock {
                                        loc: Loc::File(0, 165, 186),
                                        statements: vec![YulStatement::Assign(
                                            Loc::File(0, 167, 184),
                                            vec![YulExpression::Variable(Identifier {
                                                loc: Loc::File(0, 167, 168),
                                                name: "i".to_string(),
                                            })],
                                            YulExpression::FunctionCall(Box::new(
                                                YulFunctionCall {
                                                    loc: Loc::File(0, 172, 184),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 172, 175),
                                                        name: "add".to_string(),
                                                    },
                                                    arguments: vec![
                                                        YulExpression::Variable(Identifier {
                                                            loc: Loc::File(0, 176, 177),
                                                            name: "i".to_string(),
                                                        }),
                                                        YulExpression::HexNumberLiteral(
                                                            Loc::File(0, 179, 183),
                                                            "0x20".to_string(),
                                                            None,
                                                        ),
                                                    ],
                                                },
                                            )),
                                        )],
                                    },
                                    execution_block: YulBlock {
                                        loc: Loc::File(0, 187, 388),
                                        statements: vec![
                                            YulStatement::Assign(
                                                Loc::File(0, 217, 248),
                                                vec![YulExpression::Variable(Identifier {
                                                    loc: Loc::File(0, 217, 218),
                                                    name: "x".to_string(),
                                                })],
                                                YulExpression::FunctionCall(Box::new(
                                                    YulFunctionCall {
                                                        loc: Loc::File(0, 232, 248),
                                                        id: Identifier {
                                                            loc: Loc::File(0, 232, 235),
                                                            name: "add".to_string(),
                                                        },
                                                        arguments: vec![
                                                            YulExpression::Variable(Identifier {
                                                                loc: Loc::File(0, 236, 237),
                                                                name: "x".to_string(),
                                                            }),
                                                            YulExpression::FunctionCall(Box::new(
                                                                YulFunctionCall {
                                                                    loc: Loc::File(0, 239, 247),
                                                                    id: Identifier {
                                                                        loc: Loc::File(0, 239, 244),
                                                                        name: "mload".to_string(),
                                                                    },
                                                                    arguments: vec![
                                                                        YulExpression::Variable(
                                                                            Identifier {
                                                                                loc: Loc::File(0, 245, 246),
                                                                                name: "i".to_string(),
                                                                            },
                                                                        ),
                                                                    ],
                                                                },
                                                            )),
                                                        ],
                                                    },
                                                )),
                                            ),
                                            YulStatement::If(
                                                Loc::File(0, 278, 362),
                                                YulExpression::FunctionCall(Box::new(
                                                    YulFunctionCall {
                                                        loc: Loc::File(0, 281, 292),
                                                        id: Identifier {
                                                            loc: Loc::File(0, 281, 283),
                                                            name: "gt".to_string(),
                                                        },
                                                        arguments: vec![
                                                            YulExpression::Variable(Identifier {
                                                                loc: Loc::File(0, 284, 285),
                                                                name: "i".to_string(),
                                                            }),
                                                            YulExpression::HexNumberLiteral(
                                                                Loc::File(0, 287, 291),
                                                                "0x10".to_string(),
                                                                None,
                                                            ),
                                                        ],
                                                    },
                                                )),
                                                YulBlock {
                                                    loc: Loc::File(0, 293, 362),
                                                    statements: vec![YulStatement::Break(Loc::File(0, 327, 332))],
                                                },
                                            ),
                                        ],
                                    },
                            }),
                                YulStatement::VariableDeclaration(
                                    Loc::File(0, 414, 451),
                                    vec![
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 418, 425),
                                            id: Identifier {
                                                loc: Loc::File(0, 418, 419),
                                                name: "h".to_string(),
                                            },
                                            ty: Some(Identifier {
                                                loc: Loc::File(0, 422, 425),
                                                name: "u32".to_string(),
                                            }),
                                        },
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 427, 428),
                                            id: Identifier {
                                                loc: Loc::File(0, 427, 428),
                                                name: "y".to_string(),
                                            },
                                            ty: None,
                                        },
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 430, 437),
                                            id: Identifier {
                                                loc: Loc::File(0, 430, 431),
                                                name: "z".to_string(),
                                            },
                                            ty: Some(Identifier {
                                                loc: Loc::File(0, 434, 437),
                                                name: "u16".to_string(),
                                            }),
                                        },
                                    ],
                                    Some(YulExpression::FunctionCall(Box::new(
                                        YulFunctionCall {
                                            loc: Loc::File(0, 441, 451),
                                            id: Identifier {
                                                loc: Loc::File(0, 441, 449),
                                                name: "funcCall".to_string(),
                                            },
                                            arguments: vec![],
                                        },
                                    ))),
                                ),
                                YulStatement::Switch(YulSwitch {
                                    loc: Loc::File(0, 477, 714),
                                    condition: YulExpression::Variable(Identifier {
                                        loc: Loc::File(0, 484, 485),
                                        name: "x".to_string(),
                                    }),
                                    cases: vec![YulSwitchOptions::Case(
                                        Loc::File(0, 510, 620),
                                        YulExpression::NumberLiteral(
                                            Loc::File(0, 515, 516),
                                            "0".to_string(), "".to_string(),
                                            None,
                                        ),
                                        YulBlock {
                                            loc: Loc::File(0, 517, 620),
                                            statements: vec![YulStatement::FunctionCall(Box::new(
                                                YulFunctionCall {
                                                    loc: Loc::File(0, 547, 559),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 547, 553),
                                                        name: "revert".to_string(),
                                                    },
                                                    arguments: vec![
                                                        YulExpression::NumberLiteral(
                                                            Loc::File(0, 554, 555),
                                                            "0".to_string(), "".to_string(),
                                                            None,
                                                        ),
                                                        YulExpression::NumberLiteral(
                                                            Loc::File(0, 557, 558),
                                                            "0".to_string(), "".to_string(),
                                                            None,
                                                        ),
                                                    ],
                                                },
                                            ))],
                                        }
                                    )],
                                    default: Some(YulSwitchOptions::Default(
                                        Loc::File(0, 645, 714),
                                        YulBlock {
                                            loc: Loc::File(0, 653, 714),
                                            statements: vec![YulStatement::Leave(Loc::File(0, 683, 688))],
                                        }
                                    )),
                            }),
                            ],
                        },
                        dialect: Some(StringLiteral {
                            loc: Loc::File(0, 63, 71),
                            unicode: false,
                            string: "evmasm".to_string(),
                        }),
                        flags: None,
                    },
                    Statement::Assembly {
                        loc: Loc::File(0, 758, 1027),
                        block: YulBlock {
                          loc: Loc::File(0, 767, 1027),
                            statements: vec![YulStatement::FunctionDefinition(Box::new(
                                YulFunctionDefinition {
                                    loc: Loc::File(0, 794, 1005),
                                    id: Identifier {
                                        loc: Loc::File(0, 803, 808),
                                        name: "power".to_string(),
                                    },
                                    params: vec![
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 809, 820),
                                            id: Identifier {
                                                loc: Loc::File(0, 809, 813),
                                                name: "base".to_string(),
                                            },
                                            ty: Some(Identifier {
                                                loc: Loc::File(0, 816, 820),
                                                name: "u256".to_string(),
                                            }),
                                        },
                                        YulTypedIdentifier {
                                            loc: Loc::File(0, 822, 830),
                                            id: Identifier {
                                                loc: Loc::File(0, 822, 830),
                                                name: "exponent".to_string(),
                                            },
                                            ty: None,
                                        },
                                    ],
                                    returns: vec![YulTypedIdentifier {
                                        loc: Loc::File(0, 835, 841),
                                        id: Identifier {
                                            loc: Loc::File(0, 835, 841),
                                            name: "result".to_string(),
                                        },
                                        ty: None,
                                    }],
                                    body: YulBlock {
                                        loc: Loc::File(0, 866, 1005),
                                        statements:  vec![
                                            YulStatement::VariableDeclaration(
                                                Loc::File(0, 896, 940),
                                                vec![YulTypedIdentifier {
                                                    loc: Loc::File(0, 900, 901),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 900, 901),
                                                        name: "y".to_string(),
                                                    },
                                                    ty: None,
                                                }],
                                                Some(YulExpression::FunctionCall(Box::new(
                                                    YulFunctionCall {
                                                        loc: Loc::File(0, 905, 940),
                                                        id: Identifier {
                                                            loc: Loc::File(0, 905, 908),
                                                            name: "and".to_string(),
                                                        },
                                                        arguments: vec![
                                                            YulExpression::StringLiteral(
                                                                StringLiteral {
                                                                    loc: Loc::File(0, 909, 914),
                                                                    unicode: false,
                                                                    string: "abc".to_string(),
                                                                },
                                                                Some(Identifier {
                                                                    loc: Loc::File(0, 915, 918),
                                                                    name: "u32".to_string(),
                                                                }),
                                                            ),
                                                            YulExpression::FunctionCall(Box::new(
                                                                YulFunctionCall {
                                                                    loc: Loc::File(0, 920, 939),
                                                                    id: Identifier {
                                                                        loc: Loc::File(0, 920, 923),
                                                                        name: "add".to_string(),
                                                                    },
                                                                    arguments: vec![
                                                                        YulExpression::NumberLiteral(
                                                                            Loc::File(0, 924, 930),
                                                                            "3".to_string(), "".to_string(),
                                                                            Some(Identifier {
                                                                                loc: Loc::File(0, 926, 930),
                                                                                name: "u256".to_string(),
                                                                            }),
                                                                        ),
                                                                        YulExpression::NumberLiteral(
                                                                            Loc::File(0, 932, 938),
                                                                            "2".to_string(), "".to_string(),
                                                                            Some(Identifier {
                                                                                loc: Loc::File(0, 934, 938),
                                                                                name: "u256".to_string(),
                                                                            }),
                                                                        ),
                                                                    ],
                                                                },
                                                            )),
                                                        ],
                                                    },
                                                ))),
                                            ),
                                            YulStatement::VariableDeclaration(
                                                Loc::File(0, 969, 979),
                                                vec![YulTypedIdentifier {
                                                    loc: Loc::File(0, 973, 979),
                                                    id: Identifier {
                                                        loc: Loc::File(0, 973, 979),
                                                        name: "result".to_string(),
                                                    },
                                                    ty: None,
                                                }],
                                                None,
                                            ),
                                        ],
                                    }
                                },
                            ))],
                        },
                        dialect: None,
                        flags: None,
                    },
                ],
            }),
        },
    ))]);

    assert_eq!(expected_parse_tree, actual_parse_tree);

    assert_eq!(
        comments,
        vec![
            Comment::Block(Loc::File(0, 222, 231), "/* meh */".to_string()),
            Comment::Line(Loc::File(0, 588, 594), "// feh".to_string())
        ]
    );
}

#[test]
fn parse_revert_test() {
    let src = r#"
        contract TestToken {
            error BAR_ERROR();
            function foo()  {
                revert BAR_ERROR();
            }
        }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);

    let expected_parse_tree = SourceUnit(vec![SourceUnitPart::ContractDefinition(Box::new(
        ContractDefinition {
            loc: Loc::File(0, 9, 150),
            ty: ContractTy::Contract(Loc::File(0, 9, 17)),
            name: Identifier {
                loc: Loc::File(0, 18, 27),
                name: "TestToken".to_string(),
            },
            base: vec![],
            parts: vec![
                ContractPart::ErrorDefinition(Box::new(ErrorDefinition {
                    loc: Loc::File(0, 42, 59),
                    name: Identifier {
                        loc: Loc::File(0, 48, 57),
                        name: "BAR_ERROR".to_string(),
                    },
                    fields: vec![],
                })),
                ContractPart::FunctionDefinition(Box::new(FunctionDefinition {
                    loc: Loc::File(0, 73, 89),
                    ty: FunctionTy::Function,
                    name: Some(Identifier {
                        loc: Loc::File(0, 82, 85),
                        name: "foo".to_string(),
                    }),
                    name_loc: Loc::File(0, 82, 85),
                    params: vec![],
                    attributes: vec![],
                    return_not_returns: None,
                    returns: vec![],
                    body: Some(Statement::Block {
                        loc: Loc::File(0, 89, 140),
                        unchecked: false,
                        statements: vec![Statement::Revert(
                            Loc::File(0, 107, 125),
                            Some(IdentifierPath {
                                loc: Loc::File(0, 114, 123),
                                identifiers: vec![Identifier {
                                    loc: Loc::File(0, 114, 123),
                                    name: "BAR_ERROR".to_string(),
                                }],
                            }),
                            vec![],
                        )],
                    }),
                })),
            ],
        },
    ))]);

    assert_eq!(actual_parse_tree, expected_parse_tree);
}

#[test]
fn parse_byte_function_assembly() {
    let src = r#"
    contract ECDSA {
        function tryRecover() internal pure {
            assembly {
                v := byte(0, mload(add(signature, 0x60)))
            }
        }
    }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);
}

#[test]
fn parse_user_defined_value_type() {
    let src = r#"
        type Uint256 is uint256;
        contract TestToken {
            type Bytes32 is bytes32;
        }
        "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 2);

    let expected_parse_tree = SourceUnit(vec![
        SourceUnitPart::TypeDefinition(Box::new(TypeDefinition {
            loc: Loc::File(0, 9, 32),
            name: Identifier {
                loc: Loc::File(0, 14, 21),
                name: "Uint256".to_string(),
            },
            ty: Expression::Type(Loc::File(0, 25, 32), Type::Uint(256)),
        })),
        SourceUnitPart::ContractDefinition(Box::new(ContractDefinition {
            loc: Loc::File(0, 42, 109),
            ty: ContractTy::Contract(Loc::File(0, 42, 50)),
            name: Identifier {
                loc: Loc::File(0, 51, 60),
                name: "TestToken".to_string(),
            },
            base: vec![],
            parts: vec![ContractPart::TypeDefinition(Box::new(TypeDefinition {
                loc: Loc::File(0, 75, 98),
                name: Identifier {
                    loc: Loc::File(0, 80, 87),
                    name: "Bytes32".to_string(),
                },
                ty: Expression::Type(Loc::File(0, 91, 98), Type::Bytes(32)),
            }))],
        })),
    ]);

    assert_eq!(actual_parse_tree, expected_parse_tree);
}

#[test]
fn parse_no_parameters_yul_function() {
    let src = r#"
contract C {
	function testing() pure public {
		assembly {
			function multiple() -> ret1, ret2 {
				ret1 := 1
				ret2 := 2
			}
		}
	}
}
    "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);
}

#[test]
fn parse_random_doccomment() {
    let src = r#"
int  /** x */ constant /** x */ y/** dev:  */ = /** x */1 /** x */ + /** x */2/** x */;
    "#;

    let (actual_parse_tree, _) = crate::parse(src, 0).unwrap();
    assert_eq!(actual_parse_tree.0.len(), 1);
}

#[test]
fn test_libsolidity() {
    fn timeout_after<T, F>(d: Duration, f: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> T,
        F: Send + 'static,
    {
        let (done_tx, done_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let val = f();
            done_tx.send(()).expect("Unable to send completion signal");
            val
        });

        match done_rx.recv_timeout(d) {
            Ok(_) => Ok(handle.join().expect("Thread panicked")),
            Err(_) => Err(format!("Thread timeout-ed after {d:?}")),
        }
    }

    let source_delimiter = regex::Regex::new(r"====.*====").unwrap();
    let error_matcher = regex::Regex::new(r"// ----\r?\n// \w+( \d+)?:").unwrap();

    let semantic_tests = WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/solidity/test/libsolidity/semanticTests"),
    )
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
    .into_iter()
    .map(|entry| (false, entry));

    let syntax_tests = WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/solidity/test/libsolidity/syntaxTests"),
    )
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
    .into_iter()
    .map(|entry| (true, entry));

    let errors = semantic_tests
        .into_iter()
        .chain(syntax_tests)
        .map::<Result<_, String>, _>(|(syntax_test, entry)| {
            if entry.file_name().to_string_lossy().ends_with(".sol") {
                let source = match fs::read_to_string(entry.path()) {
                    Ok(source) => source,
                    Err(err) if matches!(err.kind(), std::io::ErrorKind::InvalidData) => {
                        return Ok(vec![])
                    }
                    Err(err) => return Err(err.to_string()),
                };

                let expect_error = syntax_test && error_matcher.is_match(&source);

                Ok(source_delimiter
                    .split(&source)
                    .filter(|source_part| !source_part.is_empty())
                    .map(|part| {
                        (
                            entry.path().to_string_lossy().to_string(),
                            expect_error,
                            part.to_string(),
                        )
                    })
                    .collect::<Vec<_>>())
            } else {
                Ok(vec![])
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .flatten()
        .filter_map(|(path, expect_error, source_part)| {
            let result = match timeout_after(Duration::from_secs(5), move || {
                crate::parse(&source_part, 0)
            }) {
                Ok(result) => result,
                Err(err) => return Some(format!("{:?}: \n\t{}", path, err)),
            };

            if let (Err(err), false) = (
                result.map_err(|diags| {
                    format!(
                        "{:?}:\n\t{}",
                        path,
                        diags
                            .iter()
                            .map(|diag| format!("{diag:?}"))
                            .collect::<Vec<_>>()
                            .join("\n\t")
                    )
                }),
                expect_error,
            ) {
                return Some(err);
            }

            None
        })
        .collect::<Vec<_>>();

    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
