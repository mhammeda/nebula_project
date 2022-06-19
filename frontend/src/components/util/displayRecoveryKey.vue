<template>
    <v-layout align-center justify-center>
        <v-card width="70%" elevation="0">
            <div class="overline mb-4">
                {{
                    getMessage() +
                        '(this page will automatically move on in 10 seconds):'
                }}
            </div>
            <v-card-title
                class="text-center text-h1 font-weight-bold"
            >
                {{ this.recoveryKey }}
            </v-card-title>
            <div class="overline mb-4">
                Please save this somewhere safe.
            </div>
        </v-card>
    </v-layout>
</template>

<script>
export default {
    props: [],
    data() {
        return {
            url: '/internal/users',
        }
    },

    methods: {
        getMessage() {
            if (this.$route.params.registeringAccount) {
                return 'Your account has been created, your recovery key is '
            } else {
                return 'Your password has been reset, your recovery key is '
            }
        },
    },

    beforeCreate() {
        this.recoveryKey = this.$route.params.recoveryKey
    },

    mounted() {
        setTimeout(() => {
            this.$router.push('login')
        }, 10000)
    },
}
</script>

<style scoped></style>
