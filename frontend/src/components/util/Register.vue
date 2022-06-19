<template>
    <div>
        <div class="registerForm">
            <h2>Register</h2>
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
                <password v-model="password" :strength-meter-only="true" />
                <v-text-field
                    color="secondary"
                    v-model="password2"
                    label="Confirm Password"
                    :type="'password'"
                    :rules="repeatPasswordRules"
                    outlined
                    required
                    @keydown.enter="handleSubmit"
                >
                </v-text-field>
                <div></div>
                <v-btn
                    :disabled="!valid"
                    color="secondary"
                    @click="handleSubmit"
                >
                    Register
                </v-btn>
            </v-form>
            <router-link to="/login"
                ><small>already have an account?</small></router-link
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
import Password from 'vue-password-strength-meter'
export default {
    components: { Password },
    props: [],
    data() {
        return {
            valid: true,
            username: '',
            password: '',
            password2: '',
            // rule checks on form input
            usernameRules: [v => !!v || 'Username required'],
            passwordRules: [
                v => !!v || 'Password required',
                v =>
                    (v.length > 7 && v.length < 64) ||
                    'Password Length should be between 8 and 64 characters.',
            ],
            // check passwords match
            repeatPasswordRules: [
                v => !!v || 'Password Confirmation Required',
                v => v === this.password || 'Passwords must match!',
            ],
            url: '/internal/users',
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        handleSubmit(e) {
            e.preventDefault()
            // validate the form
            this.$refs.form.validate()
            this.$http
                .post(
                    this.url,
                    {
                        username: this.username,
                        password: this.password,
                    },
                    { withCredentials: false }
                )
                .then(response => {
                    // if succesfully registered in, send to login page.
                    this.$router.push({
                        name: 'displayRecoveryKey',
                        params: {
                            recoveryKey: response.data.recoveryKey,
                            registeringAccount: true,
                        },
                    })
                })
                .catch(error => {
                    this.errorMessage = 'Problem contacting the Server'
                    this.errorSnackbar = true
                })
        },
    },
}
</script>

<style scoped>
.registerForm {
    margin: 10% 35% 10% 35%;
}
.confirmPasswordTextbox {
    width: 400px;
    display: inline-block;
}
</style>
