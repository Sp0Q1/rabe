extern crate rand;
#[allow(dead_code)]
extern crate serde;
extern crate serde_json;

use bn::*;
use utils::tools::{contains, string_to_json, usize_to_fr};

// Policy variables
const POLICY_OR: &'static str = "OR";
const POLICY_AND: &'static str = "AND";
const POLICY_ATT: &'static str = "ATT";

pub fn calc_pruned_str(_attr: &Vec<(String)>, _policy: &String) -> Option<(bool, Vec<(String)>)> {
    let _json = string_to_json(_policy);
    match _json {
        None => {
            println!("Error in policy (could not parse json): {:?}", _policy);
            return None;
        }
        Some(_json) => {
            return required_attributes(_attr, &_json);
        }
    }
}

pub fn required_attributes(
    _attr: &Vec<(String)>,
    _json: &serde_json::Value,
) -> Option<(bool, Vec<(String)>)> {
    if *_json == serde_json::Value::Null {
        println!("Error: passed null as json!");
        return None;
    } else {
        let mut _match: bool = false;
        let mut _emtpy_list: Vec<(String)> = Vec::new();
        if _json[POLICY_OR].is_array() {
            let _num_terms = _json[POLICY_OR].as_array().unwrap().len();
            if _num_terms >= 2 {
                for _i in 0usize.._num_terms {
                    let (_found, mut _list) = required_attributes(_attr, &_json[POLICY_OR][_i])
                        .unwrap();
                    _match = _match || _found;
                    if _match {
                        _emtpy_list.append(&mut _list);
                        break;
                    }
                }
                return Some((_match, _emtpy_list));
            } else {
                println!("Error: Invalid policy (OR with just a single child).");
                return None;
            }
        }
        // inner node
        else if _json[POLICY_AND].is_array() {
            let _num_terms = _json[POLICY_AND].as_array().unwrap().len();
            _match = true;
            if _num_terms >= 2 {
                for _i in 0usize.._num_terms {
                    let (_found, mut _list) = required_attributes(_attr, &_json[POLICY_AND][_i])
                        .unwrap();
                    _match = _match && _found;
                    if _match {
                        _emtpy_list.append(&mut _list);
                    }
                }
            } else {
                println!("Error: Invalid policy (AND with just a single child).");
                return None;
            }
            if !_match {
                _emtpy_list = Vec::new();
            }
            return Some((_match, _emtpy_list));
        }
        // leaf node
        else if _json[POLICY_ATT] != serde_json::Value::Null {
            match _json[POLICY_ATT].as_str() {
                Some(_s) => {
                    if contains(_attr, &_s.to_string()) {
                        return Some((true, vec![_s.to_string()]));
                    } else {
                        return Some((false, _emtpy_list));
                    }
                }
                None => {
                    println!("ERROR attribute value");
                    return None;
                }
            }
        } else {
            return None;
        }
    }
}

pub fn calc_coefficients_str(_policy: &String) -> Option<Vec<(String, Fr)>> {
    match string_to_json(_policy) {
        None => {
            println!("Error in policy: {:?}", _policy);
            return None;
        }
        Some(_json) => {
            return calc_coefficients(&_json, Fr::one());
        }
    }
}

pub fn calc_coefficients(_json: &serde_json::Value, _coeff: Fr) -> Option<Vec<(String, Fr)>> {
    let mut _result: Vec<(String, Fr)> = Vec::new();
    // leaf node
    if _json[POLICY_ATT] != serde_json::Value::Null {
        match _json[POLICY_ATT].as_str() {
            Some(_s) => {
                _result.push((_s.to_string(), _coeff));
                return Some(_result);
            }
            None => {
                println!("ERROR attribute value");
                return None;
            }
        }
    }
    // inner node
    else if _json[POLICY_AND].is_array() {
        let _len = _json[POLICY_AND].as_array().unwrap().len();
        let mut _vec = vec![Fr::one()];
        for _i in 1.._len {
            let _prev = _vec[_i - 1].clone();
            _vec.push(_prev + Fr::one());
        }
        let _this_coeff = recover_coefficients(_vec);
        for _i in 0.._len {
            match calc_coefficients(&_json[POLICY_AND][_i], _coeff * _this_coeff[_i]) {
                None => return None,
                Some(_res) => {
                    _result.extend(_res.iter().cloned());
                }
            }
        }
        return Some(_result);
    }
    // inner node
    else if _json[POLICY_OR].is_array() {
        let _len = _json[POLICY_OR].as_array().unwrap().len();
        let _this_coeff = recover_coefficients(vec![Fr::one()]);
        for _i in 0.._len {
            match calc_coefficients(&_json[POLICY_OR][_i], _coeff * _this_coeff[0]) {
                None => return None,
                Some(_res) => {
                    _result.extend(_res.iter().cloned());
                }
            }
        }
        return Some(_result);
    } else {
        return None;
    }
}

// lagrange interpolation
pub fn recover_coefficients(_list: Vec<Fr>) -> Vec<Fr> {
    let mut _coeff: Vec<Fr> = Vec::new();
    for _i in _list.clone() {
        let mut _result = Fr::one();
        for _j in _list.clone() {
            if _i != _j {
                _result = _result * ((Fr::zero() - _j) * (_i - _j).inverse().unwrap());
            }
        }
        _coeff.push(_result);
    }
    return _coeff;
}

pub fn gen_shares_str(_secret: Fr, _policy: &String) -> Option<Vec<(String, Fr)>> {
    match string_to_json(_policy) {
        None => {
            return None;
        }
        Some(_json_policy) => {
            return gen_shares_json(_secret, &_json_policy);
        }
    }
}

pub fn gen_shares_json(_secret: Fr, _json: &serde_json::Value) -> Option<Vec<(String, Fr)>> {
    let mut _result: Vec<(String, Fr)> = Vec::new();
    let mut _k = 0;
    let mut _length = 0;
    let mut _type = "";
    // leaf node
    if _json[POLICY_ATT] != serde_json::Value::Null {
        match _json[POLICY_ATT].as_str() {
            Some(_s) => {
                _result.push((_s.to_string(), _secret));
                return Some(_result);
            }
            None => {
                println!("Error (gen_shares_json): unkown attribute value");
                return None;
            }
        }
    }
    // inner node
    else if _json[POLICY_OR].is_array() {
        _type = POLICY_OR;
        _length = _json[POLICY_OR].as_array().unwrap().len();
        _k = 1;
    }
    // inner node
    else if _json[POLICY_AND].is_array() {
        _type = POLICY_AND;
        _length = _json[POLICY_AND].as_array().unwrap().len();
        _k = _length;
    }
    let shares = gen_shares(_secret, _k, _length);
    for _count in 0.._length {
        match gen_shares_json(shares[_count + 1], &_json[_type][_count]) {
            None => return None,
            Some(_items) => {
                _result.extend(_items.iter().cloned());
            }
        }
    }
    return Some(_result);
}

pub fn gen_shares(_secret: Fr, _k: usize, _n: usize) -> Vec<Fr> {
    let mut _shares: Vec<Fr> = Vec::new();
    if _k <= _n {
        // random number generator
        let _rng = &mut rand::thread_rng();
        // polynomial coefficients
        let mut _a: Vec<Fr> = Vec::new();
        for _i in 0.._k {
            if _i == 0 {
                _a.push(_secret);
            } else {
                _a.push(Fr::random(_rng))
            }
        }
        for _i in 0..(_n + 1) {
            let _polynom = polynomial(_a.clone(), usize_to_fr(_i));
            _shares.push(_polynom);
        }
    }
    return _shares;
}

#[allow(dead_code)]
pub fn recover_secret(_shares: Vec<Fr>, _policy: &String) -> Fr {
    let _coeff = calc_coefficients_str(_policy).unwrap();
    let mut _secret = Fr::zero();
    for _i in 0usize.._shares.len() {
        _secret = _secret + (_coeff[_i].1 * _shares[_i]);
    }
    return _secret;
}

pub fn polynomial(_coeff: Vec<Fr>, _x: Fr) -> Fr {
    let mut _share = Fr::zero();
    for _i in 0usize.._coeff.len() {
        _share = _share + (_coeff[_i] * _x.pow(usize_to_fr(_i)));
    }
    return _share;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_secret_sharing_or() {
        // OR
        let _rng = &mut rand::thread_rng();
        let _secret = Fr::random(_rng);
        //println!("_random: {:?}", into_dec(_secret).unwrap());
        let _shares = gen_shares(_secret, 1, 2);
        let _k = _shares[0];
        //println!("_original_secret: {:?}", into_dec(K).unwrap());
        let mut _input: Vec<Fr> = Vec::new();
        _input.push(_shares[1]);
        let _reconstruct = recover_secret(
            _input,
            &String::from(r#"{"OR": [{"ATT": "A"}, {"ATT": "B"}]}"#),
        );
        assert!(_k == _reconstruct);
    }

    #[test]
    fn test_gen_shares_json() {
        // OR
        let _rng = &mut rand::thread_rng();
        let _secret = Fr::random(_rng);
        let _policy = String::from(
            r#"{"AND": [{"ATT": "A"}, {"ATT": "B"}, {"ATT": "C"}, {"ATT": "D"}]}"#,
        );
        let _json = string_to_json(&_policy).unwrap();
        //println!("_random: {:?}", into_dec(_secret).unwrap());
        let _shares = gen_shares_json(_secret, &_json).unwrap();
        let _coeff = calc_coefficients_str(&_policy).unwrap();
        for _s in _shares {
            println!("_shares: {:?}", _s.0);
        }
        for _c in _coeff {
            println!("_coeff: {:?}", _c.0);
        }
        //assert!(_k == _reconstruct);
    }

    #[test]
    fn test_secret_sharing_and() {
        // AND
        let _rng = &mut rand::thread_rng();
        let _secret = Fr::random(_rng);
        //println!("_random: {:?}", into_dec(_secret).unwrap());
        let _shares = gen_shares(_secret, 2, 2);
        let _k = _shares[0];
        //println!("_original_secret: {:?}", into_dec(_k).unwrap());
        let mut _input: Vec<Fr> = Vec::new();
        _input.push(_shares[1]);
        _input.push(_shares[2]);
        //println!("_share1: {:?}", into_dec(_shares[1]).unwrap());
        //println!("_share2: {:?}", into_dec(_shares[2]).unwrap());
        let _reconstruct = recover_secret(
            _input,
            &String::from(r#"{"AND": [{"ATT": "A"}, {"ATT": "B"}]}"#),
        );
        //println!("_reconstructed: {:?}", into_dec(_reconstruct).unwrap());
        assert!(_k == _reconstruct);
    }

    #[test]
    fn test_pruning() {
        // a set of two attributes
        let mut _attributes: Vec<String> = Vec::new();
        _attributes.push(String::from("3"));
        _attributes.push(String::from("4"));

        let _result1 = calc_pruned_str(
            &_attributes,
            &String::from(r#"{"OR": [{"AND": [{"ATT": "1"}, {"ATT": "2"}]}, {"AND": [{"ATT": "3"}, {"ATT": "4"}]}]}"#),
        );
        let _result2 = calc_pruned_str(
            &_attributes,
            &String::from(
                r#"{"OR": [{"ATT": "3"}, {"AND": [{"ATT": "4"}, {"ATT": "5"}]}]}"#,
            ),
        );
        let _result3 = calc_pruned_str(
            &_attributes,
            &String::from(r#"{"AND": [{"AND": [{"ATT": "1"}, {"ATT": "2"}]}, {"AND": [{"ATT": "3"}, {"ATT": "4"}]}]}"#),
        );

        let (_match1, _list1) = _result1.unwrap();
        assert!(_match1 == true);
        assert!(_list1 == vec!["3".to_string(), "4".to_string()]);

        let (_match2, _list2) = _result2.unwrap();
        assert!(_match2 == true);
        assert!(_list2 == vec!["3".to_string()]);

        let (_match3, _list3) = _result3.unwrap();
        assert!(_match3 == false);
        assert!(_list3.is_empty() == true);
    }
}
