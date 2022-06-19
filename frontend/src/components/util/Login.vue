<template>
    <div>
        <div class="loginForm">
            <h2>Login</h2>
            <v-form ref="form" v-model="valid">
                <v-text-field
                    color="secondary"
                    v-model="username"
                    label="Username"
                    :rules="usernameRules"
                    outlined
                    required
                    @keydown.enter="handleSubmit"
                >
                </v-text-field>
                <v-text-field
                    color="secondary"
                    v-model="password"
                    label="Password"
                    :type="'password'"
                    :rules="passwordRules"
                    outlined
                    required
                    @keydown.enter="handleSubmit"
                >
                </v-text-field>
                <v-btn
                    :disabled="!valid"
                    color="secondary"
                    @click="handleSubmit"
                >
                    Login
                </v-btn>
            </v-form>
            <router-link to="/register"
                ><small>Dont have an account yet?</small></router-link
            >
            <br />
            <router-link to="/changePasswordForm"
                ><small>Forgotten Password</small></router-link
            >
        </div>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </div>
</template>

<script>
export default {
    // Login Page
    props: ['nextUrl'],
    // params for boxes
    data() {
        return {
            valid: true,
            username: '',
            password: '',
            // format checks on the username and password
            usernameRules: [v => !!v || 'Username required'],
            passwordRules: [v => !!v || 'Password required'],
            // our url
            url: '/internal/login',
            // message to display if error occurs
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    // methods for this page
    methods: {
        // function for submitting the login details.
        handleSubmit() {
            // validate the inputs
            this.$refs.form.validate()
            console.log('submitting...')
            if (this.password.length > 0) {
                // send post request for login endpoint to backend
                this.$http
                    .post(this.url, {
                        username: this.username,
                        password: this.password,
                    })
                    .then(response => {
                        console.log(response)
                        // set local storage variables to use in future
                        localStorage.setItem('loggedIn', 'true')
                        localStorage.setItem('userid', this.username)
                        localStorage.setItem('adminPermission', true)
                        localStorage.setItem('auth', response.headers['auth'])
                        // if succesfully set, move to next url or homepage by default.
                        if (localStorage.getItem('loggedIn') != null) {
                            if (this.$route.params.nextUrl != null) {
                                this.$router.push(this.$route.params.nextUrl)
                            } else {
                                this.$router.push('homepage')
                            }
                        }
                    })
                    .catch(error => {
                        // error handling
                        if (
                            error.response.status === 404 ||
                            error.response.status === 401
                        ) {
                            this.errorMessage =
                                'Username or Password was Incorrect'
                        } else {
                            this.errorMessage =
                                'There was an error contacting the server'
                        }

                        this.errorSnackbar = true
                    })
            }
        },
    },
}
</script>

<style scoped>
.loginForm {
    margin: 10% 35% 10% 35%;
}
.loginForm h2 {
    margin-bottom: 20px;
}
</style>
