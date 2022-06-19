<template>
    <div class="landingPageMain">
        <h1>Welcome!</h1>
        <img src="../assets/logo.png" />
        <div class="buttons">
            <v-btn
                id="loginbutton_landingPage"
                color="primary"
                depressed
                x-large
            >
                <router-link to="/login">Sign In</router-link>
            </v-btn>
            <v-btn depressed color="primary" x-large>
                <router-link to="/register">Register</router-link>
            </v-btn>
        </div>
    </div>
</template>

<script>
// Boring page, currently just contains links to the different routes.

export default {
    data() {
        return {}
    },
}
</script>

<style scoped>
/* Space out the main page */
.landingPageMain {
    margin-top: 10%;
}
/* Organise the buttons */
.buttons {
    margin: 5%;
}
.buttons a {
    color: white !important;
}
.buttons #loginbutton_landingPage {
    margin-right: 10px;
}
h1,
h2 {
    margin: 0% 0% 10px 0%;
    font-weight: bolder;
}
ul {
    list-style-type: none;
    padding: 0;
}
li {
    display: inline-block;
    margin: 0 10px;
}
a {
    color: #42b983;
}
</style>
