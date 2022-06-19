<template>
    <div class="changePasswordForm">
        <h2>Change password form</h2>
        <v-form ref="form" v-model="valid" lazy-validation>
            <div v-if="!checkLoggedIn()">
                <v-text-field
                    v-model="userName"
                    label="Username"
                    :rules="usernameRules"
                    outlined
                    required
                ></v-text-field>

                <v-text-field
                    v-model="recoveryKey"
                    label="Recovery key"
                    :rules="recoveryKeyRules"
                    outlined
                    required
                ></v-text-field>
            </div>

            <password
                v-model="password"
                label="Password"
                :type="password"
                :rule="passwordRules"
                outlined
                required
            />

            <!--
      <v-text-field
        v-model="password"
        label="Password"
        :type="'password'"
        :rules="passwordRules"
        outlined
        required
      >
      </v-text-field>
      -->
            <v-text-field
                v-model="password2"
                label="Confirm Password"
                :type="'password'"
                :rules="repeatPasswordRules"
                outlined
                required
                class="confirmPasswordTextbox"
            >
            </v-text-field>

            <br />

            <h3>{{ responseMessage }}</h3>

            <div>
                <v-btn :disabled="!valid" color="primary" @click="handleSubmit">
                    Submit
                </v-btn>
            </div>

            <div v-if="!checkLoggedIn()">
                <router-link to="/login"
                    ><small>already have an account?</small></router-link
                >
            </div>

            <div v-else>
                <router-link to="/homepage"
                    ><small>Changed your mind?</small></router-link
                >
            </div>
        </v-form>
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
            password: '',
            password2: '',
            recoveryKey: '',
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
            url: '',
            responseMessage: '',
            userName: '',
        }
    },
    methods: {
        checkLoggedIn() {
            return localStorage.getItem('loggedIn') != null
        },

        getUserName() {
            if (this.checkLoggedIn()) {
                return localStorage.getItem('userid')
            } else {
                return ''
            }
        },

        handleSubmit(e) {
            e.preventDefault()
            let itemsToSend = null

            let withCredentialsObject = null
            // validate the form
            this.$refs.form.validate()
            if (this.checkLoggedIn()) {
                this.userName = localStorage.getItem('userid')
                itemsToSend = {
                    password: this.password,
                }
                withCredentialsObject = { withCredentials: true }
            } else {
                itemsToSend = {
                    password: this.password,
                    recoveryKey: this.recoveryKey,
                }
                withCredentialsObject = { withCredentials: false }
            }

            this.url = '/internal/users/' + this.userName + '/password'

            this.$http
                .post(this.url, itemsToSend, withCredentialsObject)
                .then(response => {
                    // if succesfully changed password, depending on
                    // if logged on, routed to homepage else routed to
                    // login
                    console.log(response)
                    console.log(response.data)
                    if (this.checkLoggedIn()) {
                        this.$router.push('homepage')
                    } else {
                        this.$router.push({
                            name: 'displayRecoveryKey',
                            params: {
                                recoveryKey: response.data.recoveryKey,
                                registeringAccount: false,
                            },
                        })
                    }
                })
                .catch(error => {
                    console.error(error)
                    this.responseMessage = 'Failed to change password'
                })
        },
    },
}
</script>

<style scoped>
.changePasswordForm {
    margin: 10% 35% 10% 35%;
}

.confirmPasswordTextbox {
    width: 400px;
    display: inline-block;
}
</style>
