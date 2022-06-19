<template>
    <v-card width="1000px" shaped>
        <v-card-title>Replying to {{ item.author.username }}</v-card-title>
        <v-list>
            <v-list-item>
                <v-textarea
                    v-model="body"
                    outlined
                    placeholder="Write your reply here!"
                >
                </v-textarea>
            </v-list-item>
        </v-list>

        <v-card-actions>
            <v-btn @click="handleReply">
                reply
            </v-btn>
            <v-spacer></v-spacer>
            <v-btn @click="closeBox">
                Cancel
            </v-btn>
        </v-card-actions>
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
    </v-card>
</template>
<script>
export default {
    props: ['item'],
    data() {
        return {
            body: '',
            url: '/internal/posts',
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    methods: {
        handleReply() {
            this.$http
                .post(
                    this.url,
                    {
                        title: '',
                        community: this.item.community,
                        parentPost: this.item.id,
                        content: [
                            {
                                text: {
                                    text: this.body,
                                },
                            },
                        ],
                    },
                    { withCredentials: true }
                )
                .then(response => {
                    this.$emit('postSuccesful')
                })
                .catch(error => {
                    this.errorMessage = 'Problem contacting the server'
                    this.errorSnackbar = true
                })
        },

        closeBox() {
            this.$emit('closeBox')
        },
    },
}
</script>
